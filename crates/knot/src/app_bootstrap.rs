use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::component::Root;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use knot_messaging::QueuedNotifier;
use parking_lot::Mutex;

use crate::about_window::register_about_action;
use crate::app_state::build_agent_store;
use crate::app_support;
use crate::app_support::Activation;
use crate::app_support::AwaitingInput;
use crate::app_support::apply_visual_identity;
use crate::app_support::observe_system_appearance;
use crate::bug_report::register_report_issue_action;
use crate::command_center::CommandCenterWindow;
use crate::import_window::register_import_action;
use crate::library_window::Library;
use crate::library_window::open_library_window;
use crate::mcp_lifetime::hold_mcp_server;
use crate::menu_bar::MenuBarSnapshot;
use crate::menu_bar::MenuBarState;
use crate::quit_guard;
use crate::settings_window::open_settings_window;
use crate::window_actions::register_focused_window_actions;
use crate::window_options::manager_window_options;
use crate::workspace_manager::WorkspaceManager;

mod actions;
mod mcp_server;
mod menus;

pub(crate) use actions::*;
pub(crate) use mcp_server::start_mcp_server;
pub(crate) use menus::set_app_menus;

/// What [`open_workspace_manager`] needs to build the window, grouped so it
/// stays inside the workspace's argument-count convention.
struct WorkspaceManagerWindow {
    store:    Arc<Mutex<knot_agents::AgentStore>>,
    messages: Arc<Mutex<knot_messaging::MessageStore>>,
}

/// The application-menu actions and their key bindings.
///
/// A `MenuItem::action` only shows a shortcut beside its label if the
/// action has a binding, so without these the menu read as if Knot had
/// none.
pub(crate) fn install_actions_and_keys(settings: &knot_core::Settings,
                                       store: Arc<Mutex<knot_agents::AgentStore>>, cx: &mut App) {
    cx.on_action(quit);
    // Holds its own window handle; see
    // `about_window::register_about_action`.
    register_about_action(settings.title_font_name.clone().into(), cx);
    // Holds its own window handle, and reloads the store when it opens; see
    // `import_window::register_import_action`.
    register_import_action(Arc::clone(&store), cx);
    // Holds its own single-instance handle; see
    // `bug_report::register_report_issue_action`.
    register_report_issue_action(cx);
    cx.on_action(knot_help);
    register_focused_window_actions(cx);
    cx.on_action(hide_app);
    cx.on_action(hide_others);
    cx.on_action(show_all_windows);
    // Fixed and configurable shortcuts come from `keymap`, which is also
    // what validates a customization against the fixed ones.
    cx.bind_keys(crate::keymap::fixed_bindings().into_iter()
                                                .map(|fixed| fixed.binding));
    // The question card's own keys, scoped to the card like a component's
    // bindings rather than fixed app shortcuts: Up or Space mean something
    // only inside it (#430).
    cx.bind_keys(crate::panel_view::question_card_key_bindings());
    // Shift-Return in a menu, so a line's Shift alternate is reachable from
    // the keyboard (#475). Bound after `gpui_kit::init`, beside
    // gpui-component's own `enter`.
    cx.bind_keys(crate::workspace_window::alternate_line_key_bindings());
    crate::keymap::apply(&crate::keymap::Resolved::from_settings(&settings.keybindings),
                         cx);
    let settings_window: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    {
        let settings_window = Rc::clone(&settings_window);
        cx.on_action(move |_: &OpenSettings, cx| {
              // No reload here any more. This read from disk because
              // reopening the window with a snapshot captured at bootstrap
              // would show old values and overwrite a since-saved change -
              // and the window has no snapshot now.
              open_settings_window(&settings_window, cx);
          });
    }
}

/// Opens the workspace manager - the window the application starts in.
/// Wires the Window menu's two openers.
///
/// Registered here rather than in `install_actions_and_keys` because these
/// need the message store as well, and because what they do - raise a window
/// or open one - is this module's business rather than the keymap's.
fn register_window_actions(store: Arc<Mutex<knot_agents::AgentStore>>,
                           messages: Arc<Mutex<knot_messaging::MessageStore>>, cx: &mut App) {
    {
        let store = Arc::clone(&store);
        let messages = Arc::clone(&messages);
        cx.on_action(move |_: &OpenCommandCenter, cx| {
              CommandCenterWindow::open(Arc::clone(&store), Arc::clone(&messages), cx);
          });
    }
    crate::keymap::register_global_handlers(Arc::clone(&store), cx);
    {
        let (store, messages) = (Arc::clone(&store), Arc::clone(&messages));
        cx.on_action(move |_: &OpenPersonas, cx| {
              open_library_window(Library::Personas,
                                  Arc::clone(&store),
                                  Arc::clone(&messages),
                                  cx);
          });
    }
    {
        let (store, messages) = (Arc::clone(&store), Arc::clone(&messages));
        cx.on_action(move |_: &OpenPrompts, cx| {
              open_library_window(Library::Prompts,
                                  Arc::clone(&store),
                                  Arc::clone(&messages),
                                  cx);
          });
    }
    {
        let (store, messages) = (Arc::clone(&store), Arc::clone(&messages));
        cx.on_action(move |_: &OpenBench, cx| {
              open_library_window(Library::Bench,
                                  Arc::clone(&store),
                                  Arc::clone(&messages),
                                  cx);
          });
    }
    cx.on_action(move |_: &OpenWorkspaces, cx| {
          crate::window_registry::activate_or_open(
            crate::window_registry::WindowKey::WorkspaceManager,
            cx,
            |cx| open_manager_window(Arc::clone(&store), Arc::clone(&messages), cx),
        );
      });
}

/// Opens the workspace manager, or raises it when one is already open.
///
/// One manager window, like the Command Center
/// (`openspec/specs/window-lifecycle`). Closing it leaves the MCP server
/// running: the app holds that, not this window (`mcp_lifetime`).
fn open_workspace_manager(parts: WorkspaceManagerWindow, cx: &mut App) {
    let WorkspaceManagerWindow { store, messages } = parts;
    crate::window_registry::activate_or_open(crate::window_registry::WindowKey::WorkspaceManager,
                                             cx,
                                             move |cx| open_manager_window(store, messages, cx));
}

/// The manager window itself, reporting its handle.
fn open_manager_window(store: Arc<Mutex<knot_agents::AgentStore>>,
                       messages: Arc<Mutex<knot_messaging::MessageStore>>, cx: &mut App)
                       -> Option<gpui_kit::AnyWindowHandle> {
    let options = manager_window_options(cx);
    match cx.open_window(options, |window, cx| {
                // Every window tracks the OS appearance, so a light/dark flip
                // re-resolves the system palette and repaints.
                observe_system_appearance(window);
                // macOS leaves untitled windows out of the Window menu, which
                // is why only open workspaces were listed
                // there.
                window.set_window_title(&knot_core::l10n::t("workspace.manager"));
                let name_input = cx.new(|cx| {
                                       InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("workspace.name_placeholder"))
                                   });
                let view = cx.new(|cx| {
                                 let name_subscription =
                                     cx.subscribe(&name_input,
                                                  |_: &mut WorkspaceManager, _, event, cx| {
                                                      if matches!(event, InputEvent::Change) {
                                                          cx.notify();
                                                      }
                                                  });
                                 WorkspaceManager { store,
                                                    messages,
                                                    name_input,
                                                    workspace_dialog_id: None,
                                                    error: None,
                                                    _name_subscription: name_subscription,
                                                    drag: Default::default() }
                             });
                cx.new(|cx| Root::new(view, window, cx))
            }) {
        Ok(window) => Some(window.into()),
        Err(error) => {
            eprintln!("failed to open workspace manager: {error}");
            None
        }
    }
}

pub(crate) fn run() {
    let mut settings =
        knot_core::Settings::load().unwrap_or_else(|_| knot_core::Settings::platform_default());
    if let Err(err) = settings.init_source_folder() {
        eprintln!("failed to initialize source folder: {err}");
    }
    if let Err(err) = settings.install_default_personas() {
        eprintln!("failed to install default personas: {err}");
    }
    // Before anything runs git: a repository whose own configuration names
    // programs runs no git until the user trusts it.
    crate::repository_trust::install(settings.trusted_repositories.clone());
    quit_on_terminal_signals();
    // The one settings surface for the process, built before anything that
    // reads it. The MCP server thread starts below and outlives every window,
    // so it has to be the same surface the windows get rather than a second
    // one built from a clone.
    let settings = knot_core::SharedSettings::new(settings);
    let store = Arc::new(Mutex::new(build_agent_store(&settings.read())));
    let notifier = Arc::new(QueuedNotifier::new());
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let awaiting_input = Arc::new(Mutex::new(Vec::new()));
    let activation = Arc::new(Mutex::new(Vec::new()));
    let autopilot = Arc::new(Mutex::new(Vec::new()));
    let confirmations = knot_mcp_tools::ConfirmationQueue::default();
    let queues = crate::app_support::McpQueues { awaiting_input: Arc::clone(&awaiting_input),
                                                 activation:     Arc::clone(&activation),
                                                 autopilot:      Arc::clone(&autopilot),
                                                 confirmations:  Arc::clone(&confirmations), };
    // One registry for the process, built before the MCP server because both
    // it and every workspace window write to the same one.
    let subagents: crate::app_support::SubagentRegistryHandle = Arc::default();
    // Before the server, which authenticates against it, and before any
    // window, which issues agent credentials into it. The client token is
    // read from the Keychain once per launch (D1).
    let mcp_credentials = crate::mcp_credentials::McpCredentials::default();
    let mcp_client_token =
        crate::mcp_credentials::load_or_create(&crate::mcp_credentials::PlatformTokenStore,
                                               &mcp_credentials.registry);
    let (mcp_stop, mcp_status) =
        start_mcp_server(Arc::clone(&store),
                         settings.clone(),
                         Arc::clone(&notifier),
                         Arc::clone(&messages),
                         queues,
                         crate::app_support::McpShared { subagents:   Arc::clone(&subagents),
                                                         credentials: mcp_credentials.clone(), });

    gpui_kit::application()
                           // `Assets` only embeds gpui-component's own curated icon subset; our
                           // settings-window icon buttons (folder-open/pencil/trash/x/plus/copy)
                           // aren't in it, so `Icon::path(...)` silently resolved to nothing and
                           // rendered invisible. `AllAssets` embeds the complete Lucide catalog,
                           // and `KnotAssets` layers Knot's own assets over it - the panel's
                           // working animation only decodes as an animation when it arrives
                           // through this source rather than as raw bytes.
                           .with_assets(app_support::KnotAssets)
                           .run(move |cx| {
                               // Before `set_app_menus`: AppKit labels the
                               // application menu from the process name.
                               app_support::set_process_name(&knot_core::l10n::t("app.name"));
                               gpui_kit::init(cx);
                               // Every window absorbs its activating click
                               // (#309); registered before the first opens.
                               crate::first_click::install(cx);
                               // A link's URL beside the pointer (#326), in
                               // every window.
                               crate::link_tooltip::install(cx);
                               // Before every other global and before the
                               // first window: they read settings, and a
                               // window that opened without the surface
                               // installed would be holding nothing to
                               // read. See `settings_global`.
                               crate::settings_global::install_handle(settings.clone(), cx);
                               // Before the first window: a stored Light or
                               // Dark has to be in the first frame, not
                               // arrive as a repaint after one.
                               cx.set_global(crate::appearance::AppearancePreference(
                settings.read().appearance_mode,
            ));
                               crate::appearance::apply(None, cx);
                               apply_visual_identity(&settings.read(), cx);

                               // Before `on_action(quit)`: the guard reads
                               // the store through this global, and a quit
                               // arriving without it would be waved
                               // through unguarded.
                               cx.set_global(quit_guard::QuitGuard::new(Arc::clone(&store)));
                               install_actions_and_keys(&settings.read(), Arc::clone(&store), cx);
                               // The Agents menu starts with nothing
                               // selected, and so disabled; a workspace
                               // window claims it once one is.
                               cx.set_global(AwaitingInput(Arc::clone(&awaiting_input)));
                               cx.set_global(Activation(Arc::clone(&activation)));
                               cx.set_global(crate::app_support::Confirmations(Arc::clone(&confirmations)));
                               cx.set_global(crate::app_support::Autopilot(Arc::clone(&autopilot)));
                               cx.set_global(crate::app_support::Subagents(Arc::clone(&subagents)));
                               cx.set_global(mcp_credentials.clone());
                               cx.set_global(crate::mcp_credentials::ClientToken(mcp_client_token.clone()));
                               // The MCP server's state, for the settings
                               // pane's row and the failure notification.
                               cx.set_global(mcp_status);
                               // The app keeps the server running, not the
                               // manager window: closing that window used
                               // to stop it under every running agent.
                               hold_mcp_server(mcp_stop, cx);
                               // Adapters are checked for updates unasked,
                               // shortly after launch and daily (#367).
                               crate::adapter_updates::install(cx);
                               // Agents start on their schedules, whether or
                               // not their window is open (#404).
                               crate::scheduled_starts::install(Arc::clone(&store),
                                                                Arc::clone(&messages),
                                                                cx);
                               cx.set_global(MenuBarState::default());
                               // Every window that can be reopened is
                               // registered here, so a second request for
                               // one raises it rather than making another.
                               crate::window_registry::WindowRegistry::install(cx);
                               register_window_actions(Arc::clone(&store),
                                                       Arc::clone(&messages),
                                                       cx);
                               set_app_menus(&MenuBarSnapshot::default(), cx);

                               crate::notification_click::install(Arc::clone(&store),
                                                                  Arc::clone(&messages),
                                                                  cx);

                               open_workspace_manager(
                WorkspaceManagerWindow {
                    store: Arc::clone(&store),
                    messages: Arc::clone(&messages),
                },
                cx,
            );
                               // After the manager, so the workspaces it
                               // lists open in front of it (#245).
                               crate::workspace_restore::reopen_workspaces(&store, &messages, cx);
                               crate::workspace_restore::record_open_workspaces_at_quit(
                Arc::clone(&store),
                cx,
            );
                               // macOS launches a non-bundled binary without
                               // making it frontmost, so without this the
                               // window opens behind whatever was already on
                               // screen. `activate` is the app-level
                               // equivalent of ordering the window front.
                               cx.activate(true);
                           });
}

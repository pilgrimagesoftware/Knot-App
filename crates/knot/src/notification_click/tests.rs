//! A click on a desktop notification (#1; `desktop-notifications`,
//! "Clicking a notification navigates to its agent"), through the handler
//! the app installs and gpui's simulated notification response.
//!
//! The agents use an agent type with no terminal process and no ACP adapter,
//! so selecting one launches nothing.

use std::sync::Arc;

use gpui_kit::SystemNotificationResponse;
use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

const INERT_AGENT_TYPE: &str = "notification-click-test-inert";

struct Fixture {
    store:     Arc<Mutex<knot_agents::AgentStore>>,
    messages:  Arc<Mutex<knot_messaging::MessageStore>>,
    /// Two workspaces of two agents each, by workspace.
    spaces:    [(Uuid, [Uuid; 2]); 2],
    _settings: TempDir,
    _folder:   TempDir,
}

fn fixture(cx: &mut TestAppContext) -> Fixture {
    let folder = TempDir::new().expect("a temporary agent folder");
    let mut store = knot_agents::AgentStore::new();
    let mut space = |name: &str| {
        let workspace = crate::tests::workspace(name);
        let id = workspace.id;
        store.add_workspace(workspace);
        let mut agent = || {
            store.create(folder.path().to_string_lossy().into_owned(),
                         knot_agents::CreateOptions { agent_type: Some(INERT_AGENT_TYPE.into()),
                                                      workspace_id: Some(id),
                                                      ..Default::default() })
        };
        (id, [agent(), agent()])
    };
    let spaces = [space("One"), space("Two")];
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let settings = TempDir::new().expect("a temporary settings root");
    let installed = knot_core::Settings::with_store_root(settings.path());
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          crate::settings_global::install(installed, cx);
          super::install(Arc::clone(&store), Arc::clone(&messages), cx);
      });
    Fixture { store,
              messages,
              spaces,
              _settings: settings,
              _folder: folder }
}

fn click(cx: &mut TestAppContext, tag: &str) {
    cx.simulate_system_notification_response(SystemNotificationResponse { tag:       tag.into(),
                                                                          action_id: None, });
    cx.run_until_parked();
}

/// The agent a workspace's window has selected, or `None` with no window.
fn selected_in(cx: &mut TestAppContext, workspace: Uuid) -> Option<Option<Uuid>> {
    cx.update(|cx| {
          WindowRegistry::workspace_view(WindowKey::Workspace(workspace), cx)
              .map(|view| view.read(cx).selected_agent_id())
      })
}

#[gpui_kit::test]
fn a_click_opens_the_agents_workspace_with_it_selected(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    let (workspace, [_, agent]) = fixture.spaces[1];
    assert_eq!(selected_in(cx, workspace), None, "no window to begin with");

    click(cx, &agent.to_string());

    assert_eq!(selected_in(cx, workspace), Some(Some(agent)));
    let (other, _) = fixture.spaces[0];
    assert_eq!(selected_in(cx, other),
               None,
               "only the agent's own workspace opens");
}

/// Its workspace already open on another agent: the same window comes
/// forward and switches to the clicked one - no second window.
#[gpui_kit::test]
fn a_click_switches_an_open_window_to_the_agent(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    let (workspace, [first, second]) = fixture.spaces[0];
    cx.update(|cx| {
          WorkspaceWindow::open_with_selection(Arc::clone(&fixture.store),
                                               Arc::clone(&fixture.messages),
                                               workspace,
                                               Some(first),
                                               cx)
      });
    cx.run_until_parked();
    let windows = cx.update(|cx| cx.windows().len());

    click(cx, &second.to_string());

    assert_eq!(selected_in(cx, workspace), Some(Some(second)));
    assert_eq!(cx.update(|cx| cx.windows().len()),
               windows,
               "no second window");
}

/// A notification about no agent - the MCP server's failure notice, or an
/// agent removed since - opens nothing.
#[gpui_kit::test]
fn a_click_about_no_agent_opens_nothing(cx: &mut TestAppContext) {
    let fixture = fixture(cx);

    click(cx, crate::mcp_status::MCP_FAILURE_NOTIFICATION_TAG);
    click(cx, &Uuid::new_v4().to_string());

    assert_eq!(cx.update(|cx| cx.windows().len()), 0);
    let _ = fixture;
}

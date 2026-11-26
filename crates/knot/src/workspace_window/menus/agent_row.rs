//! One agent row's context menu.
//!
//! Shape and enablement come from
//! [`crate::agent_menu_entries::agent_context_menu_entries`], which is pure and
//! tested against the Swift reference's ordering; this is the wiring -
//! reading the row's facts out of the store, building the submenus, and
//! running whichever entry the user picked.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::menu::PopupMenuItem;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::agent_editor::AgentEditorRequest;
use crate::agent_editor::AgentPrefill;
use crate::agent_editor::open_agent_editor;
use crate::agent_menu::markdown_label;
use crate::agent_menu_entries::AgentMenuEntry;
use crate::agent_menu_entries::AgentMenuFacts;
use crate::agent_menu_entries::AgentMenuLine;
use crate::agent_menu_entries::agent_context_menu_lines;
use crate::open_in;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::menus::agent_row_actions::run_agent_menu_action;
use crate::workspace_window::menus::alternate_line::alternate_line;

/// Everything the agent-row context menu's handlers need. Grouped so the
/// builder takes one argument instead of seven, and so the row render can
/// clone it once per row rather than capturing each piece separately.
#[derive(Clone)]
pub(crate) struct AgentMenuTargets {
    pub(crate) store:         Arc<Mutex<knot_agents::AgentStore>>,
    pub(crate) window_entity: Entity<WorkspaceWindow>,
    pub(crate) workspace_id:  Uuid,
    pub(crate) id:            Uuid,
    pub(crate) name:          String,
    pub(crate) folder:        String,
}

/// Reads an agent's menu facts, the workspaces it could move to, and its
/// markdown history out of `store`.
///
/// Pure over the store so the rules that decide the item set - detached
/// workspaces are not move targets, an agent's own workspace is not one
/// either - are testable without a window. Called when the menu opens
/// rather than when the row renders, so a menu never offers a workspace
/// that was closed since the last repaint.
pub(crate) fn agent_menu_facts(store: &knot_agents::AgentStore, id: Uuid)
                               -> (AgentMenuFacts, Vec<(Uuid, String)>, Vec<PathBuf>) {
    let Some(agent) = store.agent(id)
    else {
        return (AgentMenuFacts::default(), Vec::new(), Vec::new());
    };
    let own_workspace = store.workspaces()
                             .iter()
                             .find(|workspace| workspace.agent_ids.contains(&id))
                             .map(|workspace| workspace.id);
    // Detached workspaces live in their own windows and are not move
    // targets, matching the reference's `attachedWorkspaces`.
    let move_targets =
        store.workspaces()
             .iter()
             .filter(|workspace| store.workspace_ui(workspace.id).is_detached != Some(true))
             .filter(|workspace| Some(workspace.id) != own_workspace)
             .map(|workspace| (workspace.id, workspace.name.clone()))
             .collect::<Vec<_>>();
    let history = agent.markdown_history.clone();
    let facts = AgentMenuFacts { is_companion:           agent.is_companion,
                                 is_shell:               agent.is_shell(),
                                 has_move_targets:       own_workspace.is_some()
                                                         && !move_targets.is_empty(),
                                 has_markdown_history:   !history.is_empty(),
                                 has_config_files:
                                     crate::agent_config::has_config_files(&agent.agent_type),
                                 is_running:             agent.activated,
                                 is_panel_mode:          agent.view_mode
                                                         == knot_core::ViewMode::Panel,
                                 can_lock:               !agent.locked,
                                 can_unlock:             agent.locked,
                                 can_autopilot_off:      !agent.autopilot_off,
                                 can_autopilot_on:       agent.autopilot_off,
                                 // A setting, not a fact about the agent: the
                                 // builders add it from the settings surface.
                                 restores_conversations: false, };
    (facts, move_targets, history)
}

/// Opens the agent editor from a menu handler, notifying the workspace
/// window once the dialog is submitted.
pub(super) fn open_editor_from_menu(targets: &AgentMenuTargets, prefill: AgentPrefill,
                                    insert_after: Option<Uuid>, edit_target: Option<Uuid>,
                                    app: &mut App) {
    let window_entity = targets.window_entity.clone();
    // A companion the form creates is one the user asked for, so the window
    // shows it where the store put it. An edit is not a creation.
    let creates_companion = prefill.is_companion && edit_target.is_none();
    open_agent_editor(Arc::clone(&targets.store),
                      AgentEditorRequest { workspace_id: targets.workspace_id,
                                           prefill,
                                           insert_after,
                                           edit_target },
                      move |id, _window, app| {
                          window_entity.update(app, |view, cx| {
                                           if creates_companion {
                                               view.show_created_companion(id, cx);
                                           }
                                           // No reload here any more. The
                                           // sidebar row resolves the agent's
                                           // persona name from the shared
                                           // surface, so a persona added since
                                           // the window opened is already
                                           // there; this used to re-read from
                                           // disk because the row read a
                                           // snapshot taken at open.
                                           cx.notify();
                                       });
                      },
                      app);
}

/// Builds the agent row's context menu: the entries
/// `agent_context_menu_entries` decides on, each with its handler.
///
/// Every dialog opens through `window.defer`. A `PopupMenu` dismisses
/// itself immediately after running a handler, and a dialog opened inline
/// goes down with it; opening on the next turn of the loop lets the menu
/// finish closing first.
pub(crate) fn agent_row_context_menu(targets: &AgentMenuTargets, menu: PopupMenu,
                                     window: &mut Window, cx: &mut Context<PopupMenu>)
                                     -> PopupMenu {
    let (facts, move_targets, markdown_history, config_files) = {
        let store = targets.store.lock();
        let (facts, move_targets, markdown_history) = agent_menu_facts(&store, targets.id);
        (facts, move_targets, markdown_history, crate::agent_config::for_agent(&store, targets.id))
    };
    let settings = crate::settings_global::read(cx);
    let facts = facts.with_autopilot(settings.autopilot_enabled)
                     .with_restored_conversations(settings.restore_conversation_on_launch);
    let mut menu = menu;
    for AgentMenuLine { entry, alternate } in agent_context_menu_lines(facts) {
        menu = match entry {
            AgentMenuEntry::Separator => menu.separator(),
            AgentMenuEntry::MoveToWorkspace => {
                let targets = targets.clone();
                let move_targets = move_targets.clone();
                let move_title = AgentMenuEntry::MoveToWorkspace.label().unwrap_or_default();
                menu.submenu(move_title, window, cx, move |mut submenu, _, _| {
                        for (workspace_id, workspace_name) in &move_targets {
                            let targets = targets.clone();
                            let workspace_id = *workspace_id;
                            submenu =
                            submenu.item(PopupMenuItem::new(workspace_name.clone()).on_click(
                                move |_, _window, app| {
                                    move_agent_to_workspace(&targets, workspace_id, app);
                                },
                            ));
                        }
                        submenu
                    })
            }
            AgentMenuEntry::OpenIn => {
                let folder = targets.folder.clone();
                let open_in_title = AgentMenuEntry::OpenIn.label().unwrap_or_default();
                menu.submenu(open_in_title, window, cx, move |mut submenu, _, _| {
                        for item in open_in::open_in_entries() {
                            submenu = match item {
                                open_in::OpenInEntry::Separator => submenu.separator(),
                                open_in::OpenInEntry::App(app_entry) => {
                                    let folder = folder.clone();
                                    submenu.item(PopupMenuItem::new(app_entry.label()).on_click(
                                    move |_, _window, _app| {
                                        open_in::open_folder(app_entry, &folder);
                                    },
                                ))
                                }
                            };
                        }
                        submenu
                    })
            }
            AgentMenuEntry::MarkdownFiles => {
                let targets = targets.clone();
                let history = markdown_history.clone();
                let markdown_title = AgentMenuEntry::MarkdownFiles.label().unwrap_or_default();
                menu.submenu(markdown_title, window, cx, move |mut submenu, _, _| {
                        for file in &history {
                            let targets = targets.clone();
                            let file = file.clone();
                            // File name, not the full path: the reference
                            // labels these by `lastPathComponent`, and a
                            // full path makes the submenu unreadable.
                            let label = markdown_label(&file);
                            submenu = submenu.item(PopupMenuItem::new(label).on_click({
                                                       move |_, _window, app| {
                                                           show_agent_markdown_file(&targets,
                                                                                    &file, app);
                                                       }
                                                   }));
                        }
                        submenu
                    })
            }
            AgentMenuEntry::ConfigFiles => {
                let files = config_files.clone();
                let title = AgentMenuEntry::ConfigFiles.label().unwrap_or_default();
                menu.submenu(title, window, cx, move |mut submenu, _, _| {
                        for file in &files {
                            let candidates = file.candidates.clone();
                            submenu = submenu.item(PopupMenuItem::new(file.label.clone())
                                                       .on_click(move |_, _window, _app| {
                                                           crate::agent_config::open_config_file(
                                &candidates,
                            );
                                                       }));
                        }
                        submenu
                    })
            }
            entry => {
                let Some(label) = entry.label()
                else {
                    continue;
                };
                let targets = targets.clone();
                match alternate {
                    Some(alternate) => {
                        let alternate_label = alternate.label().unwrap_or_default();
                        menu.item(alternate_line(label,
                                                 alternate_label,
                                                 move |shift, window, app| {
                                                     let chosen =
                                                         if shift { alternate } else { entry };
                                                     run_agent_menu_action(chosen, &targets,
                                                                           window, app);
                                                 }))
                    }
                    None => menu.item(PopupMenuItem::new(label).on_click(move |_, window, app| {
                                                                   run_agent_menu_action(entry,
                                                                                         &targets,
                                                                                         window,
                                                                                         app);
                                                               })),
                }
            }
        };
    }
    menu
}

/// Moves `targets`' agent to `workspace_id`.
///
/// Shared by the row's context menu and the menu bar's Agents menu, which
/// is the point: `app-menu` requires an item to do the same thing from
/// either surface, and the way to guarantee that is one body.
///
/// Sessions belong to a window, not to an agent, so the move hands the agent
/// over rather than its process: this window tears its session down and the
/// destination starts one, back into the same conversation. Left running
/// here, the session was a process no window showed, and the destination
/// started a second one for the same agent once it noticed the agent at all
/// - which, with nothing notifying it, was only on reopening (#247).
pub(super) fn move_agent_to_workspace(targets: &AgentMenuTargets, workspace_id: Uuid,
                                      app: &mut App) {
    let id = targets.id;
    {
        let mut store = targets.store.lock();
        if !store.move_to_workspace(id, workspace_id) {
            return;
        }
        // What Restart does with "Restore last conversation" on: the
        // destination's launch resumes the session this one is about to
        // stop, rather than starting the agent over.
        if let Err(error) = store.restart_keeping_conversation(id) {
            eprintln!("failed to keep agent {id}'s conversation across the move: {error}");
        }
    }
    targets.window_entity.update(app, |view, cx| {
                             view.teardown_session(id);
                             // The moved agent may have been this window's
                             // selection, and it no longer belongs here.
                             if view.selected_agent == Some(id) {
                                 view.selected_agent = None;
                             }
                             view.persist_agents(cx);
                             cx.notify();
                         });
    // A destination with no window starts the agent when it opens, as it
    // starts every member; an open one has to be told.
    if let Some(destination) =
        WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), app)
    {
        destination.update(app, |view, cx| {
                       view.ensure_session(id, cx);
                       view.ensure_panel_session(id, cx);
                       cx.notify();
                   });
    }
}

/// Shows `file` in `targets`' agent's markdown pane. Shared by both menus,
/// as [`move_agent_to_workspace`] is.
pub(super) fn show_agent_markdown_file(targets: &AgentMenuTargets, file: &Path, app: &mut App) {
    {
        let mut store = targets.store.lock();
        if let Err(error) = store.set_markdown_panel(targets.id, file.to_path_buf(), false) {
            eprintln!("failed to show {}: {error}", file.display());
        }
    }
    targets.window_entity.update(app, |_, cx| cx.notify());
}

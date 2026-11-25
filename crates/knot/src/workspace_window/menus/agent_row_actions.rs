//! What each plain agent-row menu entry does, shared by the row's context
//! menu and the menu bar's Agents menu so an item does the same thing from
//! either (`app-menu`).

use gpui_kit::App;
use gpui_kit::Window;

use super::agent_row::AgentMenuTargets;
use super::agent_row::open_editor_from_menu;
use crate::agent_editor::AgentPrefill;
use crate::agent_menu_entries::AgentMenuEntry;
use crate::workspace_window::menus::confirm_then;

/// Runs one plain (non-submenu) menu entry.
pub(in crate::workspace_window) fn run_agent_menu_action(entry: AgentMenuEntry,
                                                         targets: &AgentMenuTargets,
                                                         window: &mut Window, app: &mut App) {
    match entry {
        AgentMenuEntry::EditAgent => {
            open_editor_from_menu(targets,
                                  AgentPrefill::default(),
                                  None,
                                  Some(targets.id),
                                  app);
        }
        // The configurable companion: same shape as "New Shell Companion"
        // but routed through the editor, so the user names it and picks
        // its folder before it exists.
        AgentMenuEntry::NewCompanion => {
            let prefill = AgentPrefill { folder: Some(targets.folder.clone()),
                                         agent_type:
                                             Some(knot_core::agent_type::SHELL.to_string()),
                                         created_by: Some(targets.id),
                                         is_companion: true,
                                         ..Default::default() };
            open_editor_from_menu(targets, prefill, Some(targets.id), None, app);
        }
        // No confirmation: unlocking undoes it, and nothing is lost either
        // way - messages refused while locked were never stored.
        // Reversible from the same menu, so no confirmation.
        AgentMenuEntry::AutopilotOff | AgentMenuEntry::AutopilotOn => {
            let off = entry == AgentMenuEntry::AutopilotOff;
            targets.store.lock().set_autopilot_off(targets.id, off);
            targets.window_entity.update(app, |view, cx| {
                                     view.persist_agents(cx);
                                     cx.notify();
                                 });
        }
        AgentMenuEntry::LockAgent | AgentMenuEntry::UnlockAgent => {
            let locked = entry == AgentMenuEntry::LockAgent;
            targets.store.lock().set_locked(targets.id, locked);
            targets.window_entity.update(app, |view, cx| {
                                     view.persist_agents(cx);
                                     cx.notify();
                                 });
        }
        AgentMenuEntry::Deactivate => {
            // No confirmation: nothing is lost that selecting the row will
            // not bring back, which is the test Restart and Remove fail.
            targets.window_entity.update(app, |view, cx| {
                                     view.deactivate_agent(targets.id);
                                     cx.notify();
                                 });
        }
        // Asks first, unlike Deactivate: the conversation it forgets is the
        // resume point selecting the row would have brought back.
        AgentMenuEntry::DeactivateWithNewSession => {
            let targets = targets.clone();
            let description = knot_core::l10n::t_with("menu.agent.confirm.deactivate_new_body",
                                                      &[("name", &targets.name)]);
            confirm_then(window,
                         app,
                         knot_core::l10n::t("menu.agent.confirm.deactivate_new_title"),
                         description,
                         move |app| {
                             targets.window_entity.update(app, |view, cx| {
                                                      view.deactivate_agent_fresh(targets.id, cx);
                                                      cx.notify();
                                                  });
                         });
        }
        AgentMenuEntry::NewShellCompanion => {
            let created = targets.store.lock().create_shell_companion(targets.id);
            if let Ok(companion) = created {
                targets.window_entity.update(app, |view, cx| {
                                         view.persist_agents(cx);
                                         view.show_created_companion(companion, cx);
                                         cx.notify();
                                     });
            }
        }
        // A fork carries the source's session so it picks the conversation
        // up; a duplicate deliberately does not.
        AgentMenuEntry::ForkAgent => {
            let source = targets.store.lock().agent(targets.id).cloned();
            let Some(source) = source
            else {
                return;
            };
            let prefill = AgentPrefill::fork_of(&source);
            open_editor_from_menu(targets, prefill, Some(targets.id), None, app);
        }
        // The store copies every setup field and names the copy off the
        // existing names; the window starts it only if its mode does (#289).
        AgentMenuEntry::DuplicateAgent => {
            targets.window_entity.update(app, |view, cx| {
                                     view.duplicate_agent(targets.id, cx);
                                 });
        }
        // The bench is edited from the settings window too; both write onto
        // the live settings surface, see `workspace_window::bench`.
        AgentMenuEntry::SaveToBench => {
            let saved = targets.window_entity
                               .read(app)
                               .save_to_bench(targets.id, app);
            if let Err(error) = saved {
                eprintln!("failed to save the agent to the bench: {error}");
            }
        }
        // Asks first, unlike Save to Bench: it closes the agent, and its
        // conversation is not kept.
        AgentMenuEntry::BenchAgent => {
            let targets = targets.clone();
            let has_companions = !targets.store.lock().companions(targets.id).is_empty();
            let body_key = if has_companions {
                "menu.agent.confirm.bench_body_companions"
            }
            else {
                "menu.agent.confirm.bench_body"
            };
            let description = knot_core::l10n::t_with(body_key, &[("name", &targets.name)]);
            confirm_then(window,
                         app,
                         knot_core::l10n::t("menu.agent.confirm.bench_title"),
                         description,
                         move |app| {
                             targets.window_entity.update(app, |view, cx| {
                                                      if let Err(error) =
                                                          view.bench_agent(targets.id, cx)
                                                      {
                                                          eprintln!("failed to bench the agent: {error}");
                                                      }
                                                      cx.notify();
                                                  });
                         });
        }
        AgentMenuEntry::AgentInfo => {
            let identity = targets.window_entity.read(app).agent_identity(targets.id);
            crate::workspace_window::agent_info::show_agent_info(window, app, &identity);
        }
        AgentMenuEntry::RegisterAgent => {
            targets.window_entity.update(app, |view, cx| {
                                     view.send_registration_prompt(targets.id);
                                     cx.notify();
                                 });
        }
        AgentMenuEntry::RestartAgent => {
            // Decided now, not on confirm: the prompt says which of the two
            // it will do, and confirming must do what it said.
            let keep = crate::settings_global::read(app).restore_conversation_on_launch;
            let body = if keep {
                "menu.agent.confirm.restart_keep_body"
            }
            else {
                "menu.agent.confirm.restart_body"
            };
            confirm_restart(window,
                            app,
                            targets,
                            "menu.agent.confirm.restart_title",
                            body,
                            keep);
        }
        AgentMenuEntry::RestartWithNewConversation => {
            confirm_restart(window,
                            app,
                            targets,
                            "menu.agent.confirm.restart_new_title",
                            "menu.agent.confirm.restart_new_body",
                            false);
        }
        AgentMenuEntry::RemoveAgent => {
            let targets = targets.clone();
            let description = knot_core::l10n::t_with("menu.agent.confirm.remove_body",
                                                      &[("name", &targets.name)]);
            confirm_then(window,
                         app,
                         knot_core::l10n::t("menu.agent.confirm.remove_title"),
                         description,
                         move |app| {
                             targets.window_entity.update(app, |view, cx| {
                                                      view.remove_agent(targets.id, cx);
                                                      cx.notify();
                                                  });
                         });
        }
        // Handled by the builder, which needs `Window`/`Context` to make
        // a submenu, or carries no action at all.
        AgentMenuEntry::Separator
        | AgentMenuEntry::MoveToWorkspace
        | AgentMenuEntry::OpenIn
        | AgentMenuEntry::MarkdownFiles
        | AgentMenuEntry::ConfigFiles => {}
    }
}

/// Asks to restart the row's agent, and restarts it on confirmation,
/// keeping its conversation or starting a new one as `keep_conversation`
/// says - the prompt named by `title_key` and `body_key` has told the user
/// which.
fn confirm_restart(window: &mut Window, app: &mut App, targets: &AgentMenuTargets,
                   title_key: &str, body_key: &str, keep_conversation: bool) {
    let targets = targets.clone();
    let description = knot_core::l10n::t_with(body_key, &[("name", &targets.name)]);
    confirm_then(window,
                 app,
                 knot_core::l10n::t(title_key),
                 description,
                 move |app| {
                     targets.window_entity.update(app, |view, cx| {
                                              view.restart_agent(targets.id, keep_conversation, cx);
                                              cx.notify();
                                          });
                 });
}

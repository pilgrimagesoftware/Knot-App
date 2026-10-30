//! "Send prompt to": hands an issue or an OpenSpec change to one of the
//! workspace's agents from the Changes view (#504).
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/
//! work-item-prompts/spec.md`. The same submenu sits in both menus of both
//! tabs, so it is built here once from a [`WorkItemRef`] and the workspace's
//! agents.

use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::{App, Context, Entity, Window};
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// What a prompt names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace_window) enum WorkItemRef {
    /// An issue, by its URL.
    Issue(String),
    /// An OpenSpec change, by its name - or, for one in a subfolder, its
    /// path from the working tree (`ChangeEntry::prompt_ref`).
    Change(String),
}

impl WorkItemRef {
    /// The prompt sent for this item - one localized sentence with the URL
    /// or name substituted.
    pub(in crate::workspace_window) fn prompt_text(&self) -> String {
        match self {
            Self::Issue(url) => {
                knot_core::l10n::t_with("changes_view.prompt.issue", &[("url", url)])
            }
            Self::Change(name) => {
                knot_core::l10n::t_with("changes_view.prompt.change", &[("name", name)])
            }
        }
    }
}

/// One agent the submenu lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace_window) struct SendTarget {
    pub(in crate::workspace_window) id:      Uuid,
    pub(in crate::workspace_window) name:    String,
    /// Whether it can take a prompt now. A stopped agent is still listed
    /// and can still be chosen - choosing it activates it first (#208) -
    /// but its label says it is inactive.
    pub(in crate::workspace_window) running: bool,
    /// Locked against agent messaging (#310). Still listed and still
    /// chosen - this is the user prompting the agent, not another agent
    /// messaging it - but marked, so the user knows before sending.
    pub(in crate::workspace_window) locked:  bool,
}

impl SendTarget {
    /// The submenu item's text: the agent's name, marked when choosing it
    /// will activate it first, and when it is locked.
    pub(in crate::workspace_window) fn label(&self) -> String {
        let key = match (self.running, self.locked) {
            (true, false) => return self.name.clone(),
            (false, false) => "changes_view.send_target_inactive",
            (true, true) => "changes_view.send_target_locked",
            (false, true) => "changes_view.send_target_inactive_locked",
        };
        knot_core::l10n::t_with(key, &[("name", &self.name)])
    }
}

/// `agents` in alphabetical order by name, ignoring case - the submenu's
/// order. Ties keep their given order.
pub(in crate::workspace_window) fn send_targets(mut agents: Vec<SendTarget>) -> Vec<SendTarget> {
    agents.sort_by_key(|target| target.name.to_lowercase());
    agents
}

impl WorkspaceWindow {
    /// This workspace's agents as the submenu lists them.
    pub(in crate::workspace_window) fn prompt_targets(&self) -> Vec<SendTarget> {
        // The ids first: `workspace_agent_ids` takes the store lock itself,
        // and `parking_lot`'s mutex is not re-entrant, so asking for them
        // while holding it deadlocks the window.
        let ids = self.workspace_agent_ids();
        let agents = {
            let store = self.store.lock();
            ids.into_iter()
               .filter_map(|id| {
                   store.agent(id)
                        .map(|agent| (id, agent.name.clone(), agent.locked))
               })
               .collect::<Vec<_>>()
        };
        send_targets(agents.into_iter()
                           .map(|(id, name, locked)| SendTarget { running:
                                                                      self.can_receive_prompt(id),
                                                                  id,
                                                                  name,
                                                                  locked })
                           .collect())
    }

    /// Sends `item`'s prompt to agent `id` and nothing else, activating it
    /// first when it is not running (#208). The view stays where it is: the
    /// user is triaging a list, not leaving it, and activating an agent
    /// does not select it.
    ///
    /// A just-started panel session is still connecting, so the prompt
    /// queues behind the connection the way it would behind a busy turn.
    pub(in crate::workspace_window) fn send_work_item(&mut self, id: Uuid, item: &WorkItemRef,
                                                      cx: &mut Context<Self>)
                                                      -> bool {
        if !self.can_receive_prompt(id) {
            self.activate_agent(id, cx);
        }
        self.deliver_prompt(id, &item.prompt_text())
    }
}

/// Adds the "Send prompt to" item to `menu`: a submenu of `targets` when
/// there are any, else the item disabled.
pub(in crate::workspace_window) fn with_send_prompt(menu: PopupMenu, item: WorkItemRef,
                                                    targets: Vec<SendTarget>,
                                                    view: Entity<WorkspaceWindow>,
                                                    window: &mut Window,
                                                    cx: &mut Context<PopupMenu>)
                                                    -> PopupMenu {
    let title = knot_core::l10n::t("changes_view.send_prompt_to");
    if targets.is_empty() {
        return menu.item(PopupMenuItem::new(title).disabled(true));
    }
    menu.submenu(title, window, cx, move |mut submenu, _, _| {
            for target in &targets {
                let item = item.clone();
                let view = view.clone();
                let id = target.id;
                let entry = PopupMenuItem::new(target.label());
                let entry = if target.locked {
                    entry.icon(gpui_kit::assets::IconName::Lock)
                }
                else {
                    entry
                };
                submenu = submenu.item(entry.on_click(move |_, _, app: &mut App| {
                                                view.update(app, |view, cx| {
                                                        view.send_work_item(id, &item, cx);
                                                        cx.notify();
                                                    });
                                            }));
            }
            submenu
        })
}

#[cfg(test)]
mod tests;

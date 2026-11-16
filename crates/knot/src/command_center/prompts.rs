//! What the Command Center reads from the workspace windows and sends to them
//! for its cards (#10): which agents have a session, and quick prompts. It
//! holds no sessions itself, so both go through
//! `WindowRegistry::workspace_view`.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::Window;
#[cfg(test)]
use parking_lot::Mutex;
use uuid::Uuid;

use super::CommandCenterWindow;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

impl CommandCenterWindow {
    /// A Command Center over `store` with no poll, for tests outside this
    /// module that need its reads.
    #[cfg(test)]
    pub(crate) fn for_tests(store: Arc<Mutex<knot_agents::AgentStore>>) -> Self {
        Self { store,
               messages: Arc::new(Mutex::new(knot_messaging::MessageStore::new())),
               dashboard_sort: crate::dashboard::DashboardSort::default(),
               diff_stats: crate::diff_stats::DiffStatsCache::default(),
               read_folders: Default::default(),
               drawn_store_revision: 0,
               drawn_sessions: BTreeMap::new(),
               quick_prompts: Default::default() }
    }

    /// Whether each agent has a session, read from its workspace's window.
    /// An agent whose window is closed has none, whatever `activated` says.
    /// In-memory reads of the open windows, not I/O.
    pub(crate) fn sessions(&self, cx: &mut App) -> BTreeMap<Uuid, bool> {
        let workspaces: Vec<(Uuid, Vec<Uuid>)> =
            self.store
                .lock()
                .workspaces()
                .iter()
                .map(|workspace| (workspace.id, workspace.agent_ids.clone()))
                .collect();
        let mut sessions = BTreeMap::new();
        for (workspace_id, agent_ids) in workspaces {
            let view = WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx);
            for id in agent_ids {
                let has = view.as_ref()
                              .is_some_and(|view| view.read(cx).has_session(id));
                sessions.insert(id, has);
            }
        }
        sessions
    }

    /// Enter in `id`'s card field. The Command Center holds no sessions, so
    /// it hands the text to the window of the agent's workspace; with that
    /// window closed there is nothing to send to and the text stays. With
    /// `open` (Cmd+Enter) it also opens the agent, as a tap on its card does.
    pub(super) fn send_quick_prompt(&mut self, id: Uuid, open: bool, window: &mut Window,
                                    cx: &mut Context<Self>) {
        let Some(input) = self.quick_prompts.get(id).cloned()
        else {
            return;
        };
        let text = input.read(cx).value().to_string();
        if text.trim().is_empty() {
            return;
        }
        if !self.route_quick_prompt(id, &text, cx) {
            return;
        }
        input.update(cx, |field, cx| field.set_value("", window, cx));
        if open {
            // Not through `on_agent_tap`: that updates this view, which is
            // already being updated here.
            WorkspaceWindow::open_agent(Arc::clone(&self.store),
                                        Arc::clone(&self.messages),
                                        id,
                                        cx);
        }
        cx.notify();
    }

    /// Hands `text` to `id`'s workspace window as a quick prompt, saying
    /// whether it was taken. `false` with the window closed, or when it has
    /// no session for the agent that can take it.
    pub(crate) fn route_quick_prompt(&self, id: Uuid, text: &str, cx: &mut App) -> bool {
        let workspace_id = self.store
                               .lock()
                               .workspaces()
                               .iter()
                               .find(|workspace| workspace.agent_ids.contains(&id))
                               .map(|workspace| workspace.id);
        workspace_id.and_then(|workspace_id| {
                        WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                    })
                    .is_some_and(|view| view.update(cx, |view, _| view.quick_prompt(id, text)))
    }
}

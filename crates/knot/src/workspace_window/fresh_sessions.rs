//! Deactivating with a new session (#475): the window's half of
//! `AgentStore::deactivate_fresh`, for one agent and for a workspace.

use gpui_kit::App;
use uuid::Uuid;

use super::WorkspaceWindow;

impl WorkspaceWindow {
    /// Deactivates `id` and forgets its conversation, so selecting it later
    /// starts a new session (`agent-lifecycle`: "Deactivate with New
    /// Session"). Tears down every session the cascade stopped, as
    /// [`Self::deactivate_agent`] does, and persists: the cleared ids are
    /// what the next launch would otherwise resume.
    pub(super) fn deactivate_agent_fresh(&mut self, id: Uuid, cx: &App) {
        let deactivated = self.store.lock().deactivate_fresh(id);
        for agent_id in deactivated {
            self.teardown_session(agent_id);
        }
        self.persist_agents(cx);
    }

    /// Deactivate with New Session on every running agent in the workspace
    /// (`agent-list-ui`: "Shift alternates in the sidebar background
    /// menu"). One that is not running is left alone, as Deactivate All
    /// leaves it.
    pub(super) fn deactivate_all_agents_fresh(&mut self, cx: &App) {
        // The ids first: `workspace_agent_ids` takes the store lock itself.
        let ids = self.workspace_agent_ids();
        let running = {
            let store = self.store.lock();
            ids.into_iter()
               .filter(|id| store.agent(*id).is_some_and(|agent| agent.activated))
               .collect::<Vec<_>>()
        };
        for id in running {
            let deactivated = self.store.lock().deactivate_fresh(id);
            for agent_id in deactivated {
                self.teardown_session(agent_id);
            }
        }
        self.persist_agents(cx);
    }
}

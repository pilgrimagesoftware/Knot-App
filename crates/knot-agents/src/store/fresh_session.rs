//! Starting an agent's next session over: forgetting its conversation, and
//! keeping a connection that predates that from writing it back.
//!
//! [`AgentStore::restart`] and [`AgentStore::deactivate_fresh`] both forget
//! the conversation through [`forget_conversation`], so the two clear the
//! same fields. Both also replace the agent's `restart_token`, which is what
//! [`AgentStore::set_acp_session_id_if_current`] checks a late session id
//! against.

use uuid::Uuid;

use super::AgentStore;
use crate::agent::Agent;

/// Clears every session id and launch instruction tied to `agent`'s current
/// conversation, so its next start begins a new one.
pub(super) fn forget_conversation(agent: &mut Agent) {
    agent.session_id = None;
    agent.acp_session_id = None;
    agent.resume_session_id = None;
    agent.fork_session = false;
    // A fresh conversation is what a dispatched task asked for (#331).
    agent.fresh_context_requested = false;
}

impl AgentStore {
    /// Deactivates `id` as [`Self::deactivate`] does, after forgetting its
    /// conversation, so its next start is a new session rather than a
    /// resume (`agent-lifecycle`: "Deactivate with New Session").
    ///
    /// Unlike [`Self::restart`] nothing is started: the agent stays stopped
    /// until it is selected. Its `restart_token` is replaced, so a
    /// connection still starting from before cannot write its session id
    /// back. Companions are deactivated by the cascade; they are shells and
    /// have no conversation to forget. Returns what [`Self::deactivate`]
    /// returns, or nothing for an unknown id.
    pub fn deactivate_fresh(&mut self, id: Uuid) -> Vec<Uuid> {
        let Some(agent) = self.agent_mut(id)
        else {
            return Vec::new();
        };
        forget_conversation(agent);
        agent.restart_token = Uuid::new_v4();
        self.deactivate(id)
    }

    /// Stores `session_id` as `id`'s ACP session id, but only while the
    /// agent's `restart_token` is still `token`, the one its connection was
    /// started under. Returns whether it was stored.
    ///
    /// A connection reports its session id when it finishes connecting,
    /// which can be after the user restarted the agent or deactivated it
    /// with a new session. Stored then, the id the user discarded would be
    /// resumed on the next start. A plain Deactivate keeps the token, so an
    /// id arriving after one is still stored: that agent keeps its resume
    /// point.
    ///
    /// There is no unguarded setter for a single agent's ACP session id, so
    /// a connection cannot record one without naming its token.
    pub fn set_acp_session_id_if_current(&mut self, id: Uuid, token: Uuid, session_id: String)
                                         -> bool {
        match self.agent_mut(id) {
            Some(agent) if agent.restart_token == token => {
                agent.acp_session_id = Some(session_id);
                true
            }
            _ => false,
        }
    }
}

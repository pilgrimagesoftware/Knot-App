//! Starting an agent on a fresh conversation for a dispatched task (#331).
//!
//! `dispatch-task` runs on the MCP server's thread and cannot restart a
//! session, so it flags the agent instead
//! (`AgentStore::request_fresh_context`) and this window, which owns the
//! session, carries it out. Not at once: an agent still working on its
//! previous step would lose that work, so the restart waits until it is not
//! Working or awaiting input. The task meanwhile waits in its inbox, and the
//! inbox nudge holds off until the restart has happened, so the agent reads
//! it in the new conversation rather than the old one.

use gpui_kit::Context;
use knot_agents::AgentState;
use uuid::Uuid;

use super::{WorkspaceWindow, workspace_agent_ids};

impl WorkspaceWindow {
    /// Restarts, without their conversations, this workspace's agents that
    /// asked for a fresh one and are free to go, returning whether any were.
    pub(super) fn restart_for_fresh_context(&mut self, cx: &mut Context<Self>) -> bool {
        let ids = {
            // The free function, under this one lock: the method takes the
            // store lock itself, and `parking_lot` locks are not reentrant.
            let store = self.store.lock();
            let messages = self.messages.lock();
            workspace_agent_ids(&store, self.workspace_id).into_iter()
                                                          .filter(|id| {
                                                              store.agent(*id)
                                                                   .is_some_and(ready_for_fresh_context)
                                                              // A brief read and not answered is work
                                                              // in hand: wait for the reply (#405).
                                                              && !messages.has_unanswered(*id)
                                                          })
                                                          .collect::<Vec<Uuid>>()
        };
        if ids.is_empty() {
            return false;
        }
        self.restart_agents(&ids, false, cx);
        true
    }
}

/// Whether `agent` asked for a fresh conversation and has no turn under way
/// that a restart would cut short.
pub(super) fn ready_for_fresh_context(agent: &knot_agents::Agent) -> bool {
    agent.fresh_context_requested && !matches!(agent.state, AgentState::Running | AgentState::Input)
}

#[cfg(test)]
mod tests;

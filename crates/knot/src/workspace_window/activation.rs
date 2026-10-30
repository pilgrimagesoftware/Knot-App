//! Keeping "activated" and "running" the same thing (#316, #320).
//!
//! The sidebar lights an agent from its `activated` flag, and a message to an
//! agent that is already activated asks for nothing to be started - the flag
//! says it is running. But the flag is set in more places than sessions are
//! started: an agent created over MCP is `Active`, and so activated, from
//! birth, and a companion inherits its owner's flag, with nothing starting
//! either until the user happened to click its row. Such an agent looked
//! awake, took messages, and did nothing with them.
//!
//! Rather than chase every place the flag is set, the repaint poll keeps the
//! two in step: an agent of this window's workspace that is activated and has
//! no session gets one. Each is started once per activation - a session that
//! cannot start is not retried every tick - and the claim is forgotten when
//! the agent is deactivated or torn down, so the next activation starts it
//! again.

use std::collections::BTreeSet;

use gpui_kit::Context;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// One agent as the sweep sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AgentRun {
    pub(super) id:          Uuid,
    pub(super) activated:   bool,
    /// A terminal session or a panel session slot, in any phase - a failed
    /// connection is a session the user retries, not one to start again.
    pub(super) has_session: bool,
}

/// The agents to start - activated, without a session, and not started by an
/// earlier sweep - and `requested` brought up to date: claims of agents that
/// are no longer activated, or no longer present, are dropped so their next
/// activation starts them again.
pub(super) fn agents_to_start(agents: &[AgentRun], requested: &mut BTreeSet<Uuid>) -> Vec<Uuid> {
    requested.retain(|id| {
                 agents.iter()
                       .any(|agent| agent.id == *id && agent.activated)
             });
    agents.iter()
          .filter(|agent| agent.activated && !agent.has_session)
          .filter(|agent| requested.insert(agent.id))
          .map(|agent| agent.id)
          .collect()
}

impl WorkspaceWindow {
    /// Starts every activated agent of this workspace that has no session,
    /// and says whether any was started, for the poll's dirty check.
    pub(in crate::workspace_window) fn start_activated_agents(&mut self, cx: &mut Context<Self>)
                                                              -> bool {
        // Before the store lock: `workspace_agent_ids` takes it too, and
        // the lock is not reentrant.
        let ids = self.workspace_agent_ids();
        let agents = {
            let store = self.store.lock();
            ids.into_iter()
               .filter_map(|id| store.agent(id))
               .map(|agent| AgentRun { id:          agent.id,
                                       activated:   agent.activated,
                                       has_session: self.can_receive_prompt(agent.id), })
               .collect::<Vec<_>>()
        };
        let start = agents_to_start(&agents, &mut self.start_requested);
        for id in &start {
            self.ensure_session(*id, cx);
            self.ensure_panel_session(*id, cx);
        }
        !start.is_empty()
    }
}

#[cfg(test)]
mod tests;

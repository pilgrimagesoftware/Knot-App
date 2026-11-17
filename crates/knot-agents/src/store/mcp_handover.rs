//! The terminals that hand the user an agent's own MCP flow (#443).
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md` - "A server needing
//! attention delegates to the agent's own flow".
//!
//! Each is a shell companion carrying [`Agent::mcp_handover`], the server it
//! was opened for. The marker is what makes the terminal findable: a second
//! request for the same owner and server reuses it, and one left over from an
//! earlier window or run is recognisable as throwaway and removed.
//!
//! What the terminal runs is decided by the caller (`knot`'s MCP panel); this
//! module owns only the roster side.

use uuid::Uuid;

use super::{AgentStore, CreateOptions, RemovedAgent};
use crate::agent::Agent;
use crate::error::{AgentError, Result};

/// What [`AgentStore::open_mcp_handover`] creates when there is nothing to
/// reuse.
#[derive(Debug, Clone, Copy)]
pub struct McpHandoverRequest<'a> {
    /// The agent whose MCP flow is handed over. Never a companion.
    pub owner:   Uuid,
    /// The server the flow is for, as the owner's MCP section names it.
    pub server:  &'a str,
    /// The shell's initialization command: the agent type's MCP command.
    pub command: &'a str,
    /// The terminal's sidebar name.
    pub name:    &'a str,
}

/// The terminal a request ended up with, and whether it already existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpHandoverTerminal {
    Created(Uuid),
    Reused(Uuid),
}

impl McpHandoverTerminal {
    pub fn id(self) -> Uuid {
        match self {
            Self::Created(id) | Self::Reused(id) => id,
        }
    }
}

impl AgentStore {
    /// The terminal handing the user `owner`'s flow for `server`, creating
    /// one beside the owner only when none exists.
    ///
    /// A created terminal is a shell companion in the owner's folder that
    /// inherits the owner's activation, the same shape
    /// [`Self::create_shell_companion`] builds, plus the command and marker.
    pub fn open_mcp_handover(&mut self, request: McpHandoverRequest<'_>)
                             -> Result<McpHandoverTerminal> {
        let McpHandoverRequest { owner,
                                 server,
                                 command,
                                 name, } = request;
        let owner_agent = self.agent(owner).ok_or(AgentError::NotFound(owner))?;
        if owner_agent.is_companion {
            return Err(AgentError::CompanionCannotOwn(owner));
        }

        if let Some(existing) = self.mcp_handover_terminal(owner, server) {
            return Ok(McpHandoverTerminal::Reused(existing));
        }

        let (folder, activation_mode, activated) =
            (owner_agent.folder.clone(), owner_agent.activation_mode, owner_agent.activated);
        let id = self.create(folder,
                             CreateOptions { name: Some(name.to_owned()),
                                             agent_type:
                                                 Some(knot_core::agent_type::SHELL.to_owned()),
                                             shell_command: Some(command.to_owned()),
                                             created_by: Some(owner),
                                             is_companion: true,
                                             insert_after: Some(owner),
                                             activation_mode,
                                             mcp_handover: Some(server.to_owned()),
                                             ..Default::default() });
        self.set_activated(id, activated);

        Ok(McpHandoverTerminal::Created(id))
    }

    /// The handover terminal `owner` already has for `server`, if any.
    pub fn mcp_handover_terminal(&self, owner: Uuid, server: &str) -> Option<Uuid> {
        self.agents
            .iter()
            .find(|agent| {
                agent.created_by == Some(owner) && agent.mcp_handover.as_deref() == Some(server)
            })
            .map(|agent| agent.id)
    }

    /// The agent whose flow `terminal` was opened for, when it is a handover
    /// terminal at all.
    pub fn mcp_handover_owner(&self, terminal: Uuid) -> Option<Uuid> {
        self.agent(terminal)
            .filter(|agent| agent.mcp_handover.is_some())
            .and_then(|agent| agent.created_by)
    }

    /// Removes every handover terminal among `ids`, returning what went.
    ///
    /// For terminals with no session behind them: a window that has just
    /// opened has started nothing yet, and a store just loaded has no window
    /// at all, so any handover terminal there is left over and would only
    /// re-run a flow nobody asked for this time.
    pub fn remove_mcp_handovers(&mut self, ids: &[Uuid]) -> Vec<RemovedAgent> {
        let stale: Vec<Uuid> = self.agents
                                   .iter()
                                   .filter(|agent| is_handover_among(agent, ids))
                                   .map(|agent| agent.id)
                                   .collect();
        stale.into_iter().flat_map(|id| self.remove(id)).collect()
    }
}

fn is_handover_among(agent: &Agent, ids: &[Uuid]) -> bool {
    agent.mcp_handover.is_some() && ids.contains(&agent.id)
}

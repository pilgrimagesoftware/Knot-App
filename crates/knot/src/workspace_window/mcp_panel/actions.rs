//! Handing the user an agent's own MCP flow.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md` - "A server needing
//! attention delegates to the agent's own flow".
//!
//! Knot does not authenticate anything. It opens a plain shell in the agent's
//! working directory - a shell companion, the same one the sidebar's "New
//! Shell Companion" creates, which is what `acp-panel-ui`'s "Terminal remains
//! available" already guarantees - and runs that agent type's own MCP command
//! in it. The OAuth round-trip, the device code and the server's own prompts
//! all happen where they already work.
//!
//! The ACP session is untouched. The adapter subprocess is never the
//! injection target: it speaks JSON-RPC, not shell, and typing at it would
//! corrupt the session the user is trying to repair.

use knot_agents::{McpHandoverRequest, McpHandoverTerminal};
use uuid::Uuid;

use super::handover::handover;
use crate::app_state::agent_selection_for_workspace;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The binary this agent's CLI is invoked as: the user's configured
    /// command when they set one, otherwise the type's default.
    ///
    /// The configured value may carry flags meant for launching the agent
    /// (`claude --resume`); its first token is the binary, and the rest is
    /// not an argument to an MCP command.
    pub(in crate::workspace_window) fn mcp_program_for(&self, agent_type: &str,
                                                       cx: &gpui_kit::App)
                                                       -> Option<String> {
        crate::settings_global::read(cx).agent_commands
                                        .get(agent_type)
                                        .and_then(|text| text.split_whitespace().next())
                                        .map(str::to_owned)
                                        .or_else(|| {
                                            knot_core::agent_type::mcp_program(agent_type).map(str::to_owned)
                                        })
    }

    /// The terminal running this agent type's MCP flow for `server`: the
    /// one already open for that agent and server, or a new one (#443).
    ///
    /// `None` for a type with no MCP command - a row on such a type offers
    /// no action in the first place, and this is the second half of that
    /// same check - and for an agent that is gone or is itself a companion.
    pub(in crate::workspace_window) fn open_mcp_handover(&mut self, agent_id: Uuid,
                                                         server: &str, cx: &gpui_kit::App)
                                                         -> Option<Uuid> {
        let agent_type = self.store
                             .lock()
                             .agent(agent_id)
                             .map(|agent| agent.agent_type.clone())?;
        let program = self.mcp_program_for(&agent_type, cx)?;
        let flow = handover(&agent_type, &program, server)?;

        // `command` becomes the shell companion's PTY initialization
        // command, which is how the flow is entered without typing at
        // anything already running.
        let name = knot_core::l10n::t("mcp.terminal_name");
        let terminal = self.store
                           .lock()
                           .open_mcp_handover(McpHandoverRequest { owner: agent_id,
                                                                   server,
                                                                   command: flow.command(),
                                                                   name: &name })
                           .ok()?;
        if let McpHandoverTerminal::Created(_) = terminal {
            self.persist_agents(cx);
        }

        Some(terminal.id())
    }

    /// Notices a delegated terminal exiting, and asks for a fresh probe.
    ///
    /// Called from the repaint poll's exit loop, where every other
    /// consequence of a session ending is already handled, and before the
    /// terminal is removed, while its marker still names the owner. Returns
    /// whether this was one of ours, so the caller can tell a delegated
    /// terminal from an ordinary companion.
    pub(in crate::workspace_window) fn finish_mcp_handover(&mut self, exited: Uuid) -> bool {
        let owner = self.store.lock().mcp_handover_owner(exited);
        let Some(owner) = owner
        else {
            return false;
        };

        // Unconditional: a terminal closed without the user doing anything
        // re-probes to the same answer, which is cheaper than working out
        // whether anything changed and cannot leave the section stale.
        self.request_mcp_probe(owner);

        true
    }

    /// Removes this workspace's handover terminals before the window starts
    /// anything (#443).
    ///
    /// A window that is opening has no sessions yet, so any handover terminal
    /// in its workspace was left by an earlier window that closed before the
    /// terminal exited. Starting it would re-run a flow nobody asked for.
    pub(in crate::workspace_window) fn remove_stale_mcp_handovers(&mut self, cx: &gpui_kit::App) {
        let ids = self.workspace_agent_ids();
        let removed = self.store.lock().remove_mcp_handovers(&ids);
        if removed.is_empty() {
            return;
        }

        if removed.iter()
                  .any(|agent| self.selected_agent == Some(agent.id))
        {
            let store = self.store.lock();
            self.selected_agent = agent_selection_for_workspace(&store, self.workspace_id);
        }
        self.persist_agents(cx);
    }
}

#[cfg(test)]
mod tests;

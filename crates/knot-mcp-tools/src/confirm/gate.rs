//! Holding a tool call until the user has confirmed it, where D9 requires.

use knot_mcp::ToolCallResult;
use uuid::Uuid;

use super::{
    ConfirmationRequest, await_confirmation, create_agent_request, create_worktree_request,
};
use crate::McpToolCatalog;
use crate::args::optional_str;
use crate::consts;

impl McpToolCatalog {
    /// The error to answer `tool` with instead of running it when it needs
    /// the user's confirmation and did not get it; `None` to run it.
    ///
    /// The request is worked out again after the answer, and the call is
    /// refused if it changed: what runs is exactly what the user saw - a
    /// bench template edited while the dialog was open does not slip a
    /// different command through.
    pub(crate) async fn unconfirmed(&self, tool: &str, arguments: &serde_json::Value)
                                    -> Option<ToolCallResult> {
        let request = self.confirmation_request(tool, arguments)?;
        // Copied out first: a guard alive across the await would hold the
        // lock until the user answers.
        let queue = self.confirmations.lock().clone();
        let timeout = *self.confirmation_timeout.lock();
        let confirmed = await_confirmation(queue.as_ref(), request.clone(), timeout).await;
        if !confirmed {
            return Some(ToolCallResult::error(
                "Refused: the user did not confirm this call, so nothing was created. Ask the user \
                 before retrying.",
            ));
        }
        (self.confirmation_request(tool, arguments).as_ref() != Some(&request)).then(|| {
            ToolCallResult::error("Refused: the call changed while it waited for confirmation, \
                                   so nothing was created.")
        })
    }

    /// What `tool` called with `arguments` needs confirmed, if anything.
    fn confirmation_request(&self, tool: &str, arguments: &serde_json::Value)
                            -> Option<ConfirmationRequest> {
        let requester = self.requester(arguments);
        match tool {
            consts::CREATE_AGENT => {
                let bench_agents = self.bench_agents.lock();
                let bench =
                    optional_str(arguments, "benchAgentId").and_then(|id| Uuid::parse_str(id).ok())
                                                           .and_then(|id| {
                                                               bench_agents.iter().find(|bench| {
                                                                                      bench.id == id
                                                                                  })
                                                           });
                create_agent_request(arguments, bench, requester)
            }
            consts::CREATE_WORKTREE => create_worktree_request(arguments, requester),
            _ => None,
        }
    }

    /// The calling agent, as its `agentId` argument names it (the identity
    /// check has already held that to the connection), and its name.
    fn requester(&self, arguments: &serde_json::Value) -> (Uuid, String) {
        let id = optional_str(arguments, "agentId").and_then(|id| Uuid::parse_str(id).ok())
                                                   .unwrap_or_else(Uuid::nil);
        let name = self.agents
                       .lock()
                       .agent(id)
                       .map_or_else(|| id.to_string(), |agent| agent.name.clone());
        (id, name)
    }
}

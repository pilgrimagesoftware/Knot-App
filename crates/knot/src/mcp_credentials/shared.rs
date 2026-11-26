//! The registry and refusal log, shared by the MCP server's thread and every
//! window.

use knot_mcp::{AgentCredentials, CredentialRegistry, RefusalLog};
use uuid::Uuid;

/// The MCP credentials one Knot process holds. Cheap to clone; clones share
/// the registry and the refusal counts.
///
/// A global, because the server is started before `gpui_kit::application()`
/// and every workspace window launches adapters into the same registry.
#[derive(Clone, Default)]
pub(crate) struct McpCredentials {
    pub(crate) registry: CredentialRegistry,
    pub(crate) refusals: RefusalLog,
}

impl gpui_kit::Global for McpCredentials {}

impl McpCredentials {
    /// The process's credentials, or an empty set where none was installed
    /// (a test window): issuing into it works, and nothing serves it.
    pub(crate) fn global(cx: &gpui_kit::App) -> Self {
        cx.try_global::<Self>().cloned().unwrap_or_default()
    }

    /// A fresh pair for `agent`, revoking the one it held (D8).
    pub(crate) fn issue(&self, agent: Uuid) -> AgentCredentials {
        self.registry.issue_agent_pair(agent)
    }

    /// Revokes `agent`'s pair: its adapter has stopped. Its run of
    /// refusals goes too, so the next adapter starts with a clean slate.
    pub(crate) fn revoke(&self, agent: Uuid) {
        self.registry.revoke_agent(agent);
        self.refusals.forget(agent);
    }
}

/// The client token in effect, and any problem keeping it, for the MCP
/// settings tab to show, copy and regenerate. Replaced by
/// [`super::regenerate`]'s result when the user regenerates it.
#[derive(Clone)]
pub(crate) struct ClientToken(pub(crate) super::LoadedClientToken);

impl gpui_kit::Global for ClientToken {}

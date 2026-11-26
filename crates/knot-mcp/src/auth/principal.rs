//! Who an authenticated request is, and which routes each kind of
//! principal may reach (D1).

use std::fmt;

use uuid::Uuid;

/// What a credential proves about the request that presents it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Principal {
    /// A launched agent, by its tool credential.
    Agent(Uuid),
    /// A launched agent's hook scripts, by its hook credential.
    AgentHooks(Uuid),
    /// A client the user registered by hand, by the client token. Bound to
    /// no agent.
    External,
}

impl Principal {
    /// The agent this principal speaks for, when it speaks for one.
    pub fn agent(self) -> Option<Uuid> {
        match self {
            Self::Agent(id) | Self::AgentHooks(id) => Some(id),
            Self::External => None,
        }
    }

    /// Whether this principal's credential covers `route`.
    pub fn may_reach(self, route: Route) -> bool {
        match (self, route) {
            (Self::External, _) => true,
            (Self::Agent(_), Route::Mcp | Route::StatusRead) => true,
            (Self::Agent(_), Route::Info | Route::Hook) => false,
            (Self::AgentHooks(_), Route::Hook) => true,
            (Self::AgentHooks(_), Route::Info | Route::Mcp | Route::StatusRead) => false,
        }
    }
}

/// The principal as the log names it: the agent's id and which credential,
/// or `external`. Never anything secret.
impl fmt::Display for Principal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Agent(id) => write!(f, "agent {id}"),
            Self::AgentHooks(id) => write!(f, "agent-hooks {id}"),
            Self::External => f.write_str("external"),
        }
    }
}

/// The authenticated routes, grouped by what a credential must cover to
/// reach them. `/health` is not here: it needs none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// `GET /`, the server's name and version.
    Info,
    /// `/mcp`, POST and GET.
    Mcp,
    /// `GET /api/v1/agent/status`.
    StatusRead,
    /// `POST /api/v1/agent/status` and `POST /api/v1/agent/register`.
    Hook,
}

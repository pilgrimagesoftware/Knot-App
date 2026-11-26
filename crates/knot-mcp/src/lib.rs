//! Local MCP HTTP server.
//!
//! Implements `openspec/specs/mcp-server/spec.md`.

mod auth;
mod backoff;
mod consts;
mod error;
mod hooks;
mod log;
mod origin;
mod probe;
mod protocol_version;
mod rpc;
mod server;
mod session;
mod state;
mod status;
mod supervisor;
mod tools;
mod transcript;

pub use auth::{
    AgentCredentials, CredentialKind, CredentialRegistry, Principal, RefusalKind, RefusalLog,
    RefusalSource, Route, Secret,
};
pub use backoff::backoff_delay;
pub use consts::{DEFAULT_PORT, LOG_FILE_NAME};
pub use error::{McpError, Result};
pub use hooks::{
    AgentHookHandler, HookError, HookRequest, HookStatus, claude_status, codex_turn_complete,
    extract_metadata, last_assistant_message_from_transcript,
};
pub use log::{Entry, Level, Logger, Subject, describe_call};
pub use probe::{ProbeFailures, probe_health};
pub use rpc::{JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, dispatch};
pub use server::{AgentsSnapshotFn, McpServer};
pub use session::{McpSession, McpSessionManager};
pub use state::ServerState;
pub use status::{AgentStatusEntry, agent_status};
pub use supervisor::{Supervisor, SupervisorTuning};
pub use tools::{
    Caller, EmptyCatalog, PropertySchema, ToolCallResult, ToolCatalog, ToolContent, ToolDefinition,
    ToolInputSchema,
};
pub use transcript::TranscriptRoot;

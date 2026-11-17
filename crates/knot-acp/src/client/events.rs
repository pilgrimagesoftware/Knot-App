use crate::error::SessionEndCause;
use crate::protocol::{ConfigOption, PermissionRequest, QuestionRequest, SessionUpdate};

#[derive(Debug)]
pub enum SessionEvent {
    Update(SessionUpdate),
    PermissionRequest(PermissionRequest),
    /// The agent asks the user to fill in a form (`elicitation/create`):
    /// Claude Code's AskUserQuestion, or an MCP server's input request.
    Question(QuestionRequest),
    /// The permission request or form with this `rpc_id` has been answered
    /// and is no longer pending - by the user, by Stop, by the session
    /// ending, or withdrawn by the agent. Sent for every request, however it
    /// was answered, so a consumer showing it as pending can stop (#344).
    InputResolved {
        rpc_id: serde_json::Value,
    },
    Ended(SessionEndCause),
}

#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub session_id:     String,
    pub config_options: Vec<ConfigOption>,
}

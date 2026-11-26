pub const REGISTER_AGENT: &str = "register-agent";
pub const LIST_AGENTS: &str = "list-agents";
pub const DESCRIBE_AGENTS: &str = "describe-agents";
pub const SEND_MESSAGE: &str = "send-message";
pub const CHECK_MESSAGES: &str = "check-messages";
pub const BROADCAST_MESSAGE: &str = "broadcast-message";
pub const LIST_REPOS: &str = "list-repos";
pub const LIST_WORKTREES: &str = "list-worktrees";
pub const CREATE_AGENT: &str = "create-agent";
pub const CLOSE_AGENT: &str = "close-agent";
pub const CREATE_WORKTREE: &str = "create-worktree";
pub const SET_STATUS: &str = "set-status";
pub const SET_WORKING_DIRECTORY: &str = "set-working-directory";
pub const DISPLAY_MARKDOWN: &str = "display-markdown";
pub const VIEW_MERMAID: &str = "view-mermaid";
pub const SHOW_CHECKLIST: &str = "show-checklist";
pub const GET_CHECKLIST: &str = "get-checklist";
pub const PLAN_TASKS: &str = "plan-tasks";
pub const DISPATCH_TASK: &str = "dispatch-task";
pub const COMPLETE_TASK: &str = "complete-task";
pub const TASK_STATUS: &str = "task-status";

/// How long an MCP call waits for the user to confirm it (D9) before it is
/// refused. Long enough to read the dialog; short enough that an agent
/// whose user is away gets an answer it can act on.
pub const CONFIRMATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

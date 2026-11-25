//! Confirmation for MCP calls that would run a command the user did not
//! write or put a worktree far from its repository
//! (`openspec/changes/mcp-auth/design.md` D9).
//!
//! Authentication says *which* agent is calling, not that the user wants
//! the effect. So `create-agent` with a shell command, and any worktree
//! created outside the repository's parent directory, wait for the user to
//! confirm a dialog naming the requesting agent, the command and the
//! folder. A denial, or no answer within the timeout, is a tool error and
//! nothing is created.
//!
//! [`rule`] decides which calls need it, [`queue`] carries a request to the
//! UI and its answer back, and [`gate`] holds a call until it has one.

mod gate;
mod queue;
mod rule;

pub use queue::{ConfirmationQueue, PendingConfirmation, await_confirmation};
pub use rule::ConfirmationRequest;
pub(crate) use rule::{create_agent_request, create_worktree_request};

#[cfg(test)]
mod tests;

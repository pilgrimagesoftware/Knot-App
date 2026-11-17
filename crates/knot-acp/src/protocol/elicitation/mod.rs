//! `elicitation/create` in form mode: the agent asking the user to fill in a
//! form, which is how `claude-agent-acp` presents Claude Code's
//! AskUserQuestion tool and forwards an MCP server's own input requests.
//!
//! The schema is MCP's restricted elicitation subset - a flat object of
//! primitive properties - parsed at the boundary into [`QuestionRequest`]
//! ([`parse`]); a form outside that subset is refused there and never reaches
//! a caller. [`FormAnswer`] is the caller's reply ([`answer`]).
//!
//! Contract: `acp-client`'s "Form elicitation requests", "Form answers are
//! typed per field" and "Unrenderable forms are refused". Wire shapes:
//! <https://agentclientprotocol.com/protocol/v1/elicitation> and MCP's
//! <https://modelcontextprotocol.io/specification/2025-06-18/client/elicitation>.

mod answer;
mod form;
mod parse;

pub use answer::{AnswerValue, FormAnswer};
pub use form::{Choice, FieldKind, FormField, QuestionRequest};
// The tests assert which refusal a form gets; callers only see its message.
#[cfg(test)]
use parse::Refusal;
pub(crate) use parse::parse_form;

#[cfg(test)]
mod tests;

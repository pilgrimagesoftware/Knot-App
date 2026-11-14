//! Reading `agy acp`'s tool calls for delegations.
//!
//! Pinned to agy 1.5.8, captured in `tests/fixtures/antigravity/`. A
//! delegation is the tool `invoke_subagent`, whose `rawInput` is
//! `{ "Prompt", "Role", "TypeName" }`. The tool's name also appears as `name`
//! and in `title` ("Run invoke_subagent"), but a decoded update carries
//! neither to a recognizer, and `_meta` is empty, so this keys on the input's
//! three keys, which no other agy tool was seen to share.
//!
//! The failure arm uses ACP's generic `failed` status; a failed delegation
//! has not been observed.

use crate::event::SubagentEvent;
use crate::recognize::recognizer::Recognizer;
use crate::recognize::report::ToolCallReport;

const OPEN_STATUSES: &[&str] = &["pending", "in_progress"];
const STATUS_COMPLETED: &str = "completed";
const STATUS_FAILED: &str = "failed";

pub struct AntigravityRecognizer;

impl AntigravityRecognizer {
    fn is_delegation(report: &ToolCallReport<'_>) -> bool {
        ["Prompt", "Role", "TypeName"].iter()
                                      .all(|key| report.input_str(key).is_some())
    }
}

impl Recognizer for AntigravityRecognizer {
    fn recognize(&self, report: &ToolCallReport<'_>) -> Option<SubagentEvent> {
        if !Self::is_delegation(report) {
            return None;
        }

        match report.status {
            Some(STATUS_COMPLETED) => SubagentEvent::completed(report.id, "succeeded", None),
            Some(STATUS_FAILED) => {
                SubagentEvent::completed(report.id, "failed", report.result_text)
            }
            Some(status) if OPEN_STATUSES.contains(&status) => dispatch(report),
            None => dispatch(report),
            Some(_) => None,
        }
    }
}

/// `TypeName` is the subagent's kind and `Role` its one-line task; `Prompt`
/// is the whole instruction and too long for a row.
fn dispatch(report: &ToolCallReport<'_>) -> Option<SubagentEvent> {
    SubagentEvent::dispatched(report.id,
                              report.input_str("TypeName"),
                              report.input_str("Role"))
}

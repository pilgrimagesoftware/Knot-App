//! The Antigravity recognizer, driven off fixtures captured from a live
//! `agy acp` 1.5.8 session. See `tests/fixtures/antigravity/README.md`.

use knot_subagents::recognize::{ToolCallReport, for_agent_type};
use knot_subagents::{Outcome, SubagentEvent, SubagentId};
use serde_json::Value;

const DISPATCH: &str = include_str!("fixtures/antigravity/dispatch.json");
const COMPLETION: &str = include_str!("fixtures/antigravity/completion.json");
const ORDINARY: &str = include_str!("fixtures/antigravity/not_a_delegation.json");

fn update(fixture: &str) -> Value {
    let params: Value = serde_json::from_str(fixture).expect("fixture is valid JSON");

    params.get("update").expect("fixture has an update").clone()
}

fn recognize(fixture: &str) -> Option<SubagentEvent> {
    let update = update(fixture);
    let report = ToolCallReport { id:          update["toolCallId"].as_str().unwrap(),
                                  meta:        update.get("_meta"),
                                  raw_input:   update.get("rawInput"),
                                  status:      update.get("status").and_then(Value::as_str),
                                  result_text: None, };

    for_agent_type("antigravity").expect("a recognizer")
                                 .recognize(&report)
}

#[test]
fn invoke_subagent_is_a_dispatch_with_its_role_as_the_task() {
    let Some(SubagentEvent::Dispatched { id, task, .. }) = recognize(DISPATCH)
    else {
        panic!("expected a dispatch");
    };

    assert_eq!(id, SubagentId::new("call_565704519397567561"));
    assert_eq!(task, "Directory Explorer");
}

#[test]
fn the_completion_closes_the_same_subagent() {
    let Some(SubagentEvent::Completed { id, outcome, .. }) = recognize(COMPLETION)
    else {
        panic!("expected a completion");
    };

    assert_eq!(id, SubagentId::new("call_565704519397567561"));
    assert_eq!(outcome, Outcome::Succeeded);
}

#[test]
fn an_ordinary_tool_call_is_not_a_delegation() {
    assert_eq!(recognize(ORDINARY), None);
}

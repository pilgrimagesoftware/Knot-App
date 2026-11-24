//! Tests for how tool-call events build, update and diff a card.

use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;
use knot_acp::ToolCallContent;
use serde_json::Value;
use serde_json::json;

use super::fixture::*;
use crate::panel_state::*;

/// A start carrying the two fields an adapter identifies a call by.
fn tool_call_start_with(id: &str, raw_input: Value, meta: Value) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         "think".to_string(),
                                                        title:        String::new(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(),
                                                        raw_input:    Some(raw_input),
                                                        meta:         Some(meta), })
}

#[test]
fn tool_call_lifecycle_builds_one_card() {
    let mut state = PanelState::new();

    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));
    state.apply(tool_call_update("tc1",
                                 Some("completed"),
                                 vec![ToolCallContent::Text("done".to_string())]));

    assert_eq!(
               state.messages,
               vec![PanelMessage::ToolCall(Box::new(
        ToolCallCard { id:        "tc1".to_string(),
                       kind:      "execute".to_string(),
                       title:     String::new(),
                       status:    "completed".to_string(),
                       content:   vec![ToolCallContent::Text("done".to_string())],
                       raw_input: None,
                       meta:      None, }
    ))]
    );
}

/// The two fields a recognizer reads reach the card, unparsed.
#[test]
fn a_tool_call_card_carries_raw_input_and_meta() {
    let mut state = PanelState::new();

    state.apply(tool_call_start_with("tc1",
                                     json!({ "subagent_type": "discovery" }),
                                     json!({ "claudeCode": { "subagent": true } })));

    let card = card(&state, 0);
    assert_eq!(card.raw_input
                   .as_ref()
                   .and_then(|input| input.get("subagent_type")),
               Some(&json!("discovery")));
    assert_eq!(card.meta
                   .as_ref()
                   .and_then(|meta| meta.pointer("/claudeCode/subagent")),
               Some(&json!(true)));
}

/// The defect this guards against is specific and quiet. Claude Code's
/// adapter stamps `subagent: true` on the *start* and sends only `toolName`
/// on the finish, so overwriting `_meta` unconditionally would erase the
/// marker at exactly the moment a completion needs to be recognized - and the
/// subagent would sit on screen as permanently running.
#[test]
fn a_status_only_update_does_not_blank_raw_input_or_meta() {
    let mut state = PanelState::new();

    state.apply(tool_call_start_with("tc1",
                                     json!({ "subagent_type": "discovery" }),
                                     json!({ "claudeCode": { "subagent": true } })));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    let card = card(&state, 0);
    assert_eq!(card.status, "completed");
    assert!(card.raw_input.is_some(),
            "raw input was blanked by a status-only update");
    assert_eq!(card.meta
                   .as_ref()
                   .and_then(|meta| meta.pointer("/claudeCode/subagent")),
               Some(&json!(true)),
               "the subagent marker was erased by a status-only update");
}

/// A finished call with no content must not keep reading as running -
/// the card renderer keys its in-progress placeholder on this, and
/// keying it on "no output yet" instead left completed cards stuck
/// showing "Running…".
#[test]
fn a_completed_tool_call_is_finished_even_with_no_content() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert!(card.is_finished());
    assert!(!card.failed());
    assert!(card.content.is_empty());
}

/// Per the ACP spec, every field but `toolCallId` is optional in an
/// update: one carrying only content must not blank out the status.
#[test]
fn a_content_only_update_leaves_the_status_alone() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
    state.apply(tool_call_update("tc1",
                                 None,
                                 vec![ToolCallContent::Text("late output".to_string())]));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.status, "completed");
    assert_eq!(card.content,
               vec![ToolCallContent::Text("late output".to_string())]);
}

#[test]
fn edit_tool_call_attaches_a_diff() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "edit"));

    state.apply(tool_call_update("tc1",
                                 Some("completed"),
                                 vec![ToolCallContent::Diff { path:     "src/lib.rs".to_string(),
                                                              old_text: Some("old".to_string()),
                                                              new_text: "new".to_string(), }]));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.content,
               vec![ToolCallContent::Diff { path:     "src/lib.rs".to_string(),
                                            old_text: Some("old".to_string()),
                                            new_text: "new".to_string(), }]);
}

/// The legacy `diff` session update carries no `toolCallId`, so it
/// attaches to the most recent card - kept working because
/// `acp-client`'s streaming requirement names diffs as a distinguished
/// event, even though no shipping agent emits one this way.
#[test]
fn a_standalone_diff_update_attaches_to_the_last_card() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "edit"));

    state.apply(SessionEvent::Update(SessionUpdate::Diff { path: "src/lib.rs".to_string(),
                                                           diff: "-old\n+new".to_string(), }));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.content,
               vec![ToolCallContent::Diff { path:     "src/lib.rs".to_string(),
                                            old_text: None,
                                            new_text: "-old\n+new".to_string(), }]);
}

#[test]
fn turn_end_mid_tool_call_keeps_the_cards_last_known_state() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason:
                                                                  knot_acp::StopReason::EndTurn, }));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.status, "in_progress");
    assert!(!card.is_finished());
    assert!(card.content.is_empty());
}

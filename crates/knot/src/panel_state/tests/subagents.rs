//! A subagent's updates fold into the delegation card that launched it, not
//! into the conversation (#476).
//!
//! The sequence replayed here is the reported one: the main agent streams one
//! reply holding a table while a background subagent it launched runs
//! commands. claude-agent-acp stamps the subagent's updates
//! `_meta.claudeCode.parentToolUseId` (`stampParentToolUseId`, 0.111.0), and
//! they land between the main agent's chunks - here mid-word, as in the
//! screenshot ("PSC endp" | tool call | "oint").

use knot_acp::{SessionEvent, SessionUpdate, StopReason};
use serde_json::json;

use super::fixture::{text, tool_call_update};
use crate::panel_state::{PanelMessage, PanelState, SubagentItem};

/// The id of the `Agent` call that launched the subagent.
const DELEGATION: &str = "toolu_agent";

fn subagent_meta() -> serde_json::Value {
    json!({ "claudeCode": { "parentToolUseId": DELEGATION, "toolName": "Bash" } })
}

fn delegation_start() -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: DELEGATION.to_string(),
                                                        kind:         "think".to_string(),
                                                        title:        "Provider breaking \
                                                                       changes"
                                                                               .to_string(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(),
                                                        raw_input:    None,
                                                        meta:         Some(json!({ "claudeCode": {
                                                                "toolName": "Agent",
                                                                "subagent": true } })), })
}

fn subagent_call(id: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         "execute".to_string(),
                                                        title:        "git show".to_string(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(),
                                                        raw_input:    None,
                                                        meta:         Some(subagent_meta()), })
}

fn subagent_text(value: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TextDelta { text: value.to_string(),
                                                    meta: Some(subagent_meta()), })
}

fn turn_end() -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason: StopReason::EndTurn, })
}

const TABLE: &str = "| | PGA | PSC endpoint |\n|---|---|---|\n| Scope | Every Google API | One \
                     target API per endpoint |\n";

/// The main agent's reply, split where the subagent's updates landed.
fn interleaved(state: &mut PanelState) {
    state.push_user_message("compare them".to_string());
    state.apply(delegation_start());
    state.apply(text("Two agents are working on it.\n\n| | PGA | PSC endp"));
    state.apply(subagent_call("toolu_bash_1"));
    state.apply(text("oint |\n|---|---|---|\n| Scope | Every Google API | One "));
    state.apply(subagent_text("Reading the lock "));
    state.apply(tool_call_update("toolu_bash_1", Some("completed"), Vec::new()));
    state.apply(subagent_text("files now."));
    state.apply(subagent_call("toolu_bash_2"));
    state.apply(text("target API per endpoint |\n"));
}

fn assistant_messages(state: &PanelState) -> Vec<&str> {
    state.messages
         .iter()
         .filter_map(|message| match message {
             PanelMessage::Assistant(text) => Some(text.as_str()),
             _ => None,
         })
         .collect()
}

#[test]
fn the_main_reply_stays_one_segment_with_its_table_intact() {
    let mut state = PanelState::new();
    interleaved(&mut state);

    let replies = assistant_messages(&state);
    assert_eq!(replies.len(), 1, "one main segment: {replies:?}");
    assert!(replies[0].contains(TABLE),
            "the table is whole: {:?}",
            replies[0]);
    assert!(state.messages
                 .iter()
                 .filter(|message| matches!(message, PanelMessage::ToolCall(_)))
                 .count()
            == 1,
            "only the delegation is a top-level card");
}

#[test]
fn the_subagents_calls_and_prose_nest_under_the_delegation_in_order() {
    let mut state = PanelState::new();
    interleaved(&mut state);

    let delegation = state.tool_call(DELEGATION).expect("the delegation card");
    let items: Vec<String> = delegation.subagent
                                       .iter()
                                       .map(|item| match item {
                                           SubagentItem::ToolCall(card) => {
                                               format!("call {} {}", card.id, card.status)
                                           }
                                           SubagentItem::Text(text) => format!("text {text}"),
                                       })
                                       .collect();
    assert_eq!(items,
               ["call toolu_bash_1 completed",
                "text Reading the lock files now.",
                "call toolu_bash_2 pending"]);
}

/// The subagent's update arrived before the card it belongs to.
#[test]
fn an_update_ahead_of_its_delegation_moves_in_when_the_card_arrives() {
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    state.apply(subagent_call("toolu_bash_1"));
    state.apply(text("Working."));
    state.apply(delegation_start());

    let delegation = state.tool_call(DELEGATION).expect("the delegation card");
    assert_eq!(delegation.subagent.len(), 1);
    assert_eq!(assistant_messages(&state), ["Working."]);
}

/// A delegation that never arrives in the turn: its subagent's updates are
/// shown in the conversation rather than lost.
#[test]
fn updates_whose_delegation_never_arrives_are_placed_at_the_turns_end() {
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    state.apply(subagent_call("toolu_bash_1"));
    state.apply(subagent_text("orphaned prose"));
    assert!(state.tool_call("toolu_bash_1").is_some(),
            "held, and still reachable for its own updates");

    state.apply(turn_end());

    assert!(state.messages
                 .iter()
                 .any(|message| matches!(message, PanelMessage::ToolCall(card) if card.id == "toolu_bash_1")));
    assert!(assistant_messages(&state).contains(&"orphaned prose"));
}

/// A turn the agent answered with an error ends with no `TurnEnd`; what it
/// held is placed in that turn, ahead of the next prompt (#497 review).
#[test]
fn updates_held_when_a_turn_ends_in_an_error_stay_in_that_turn() {
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    state.apply(subagent_call("toolu_bash_1"));

    state.push_error("the adapter refused the prompt".to_string());

    let card_at = state.messages
                       .iter()
                       .position(|message| {
                           matches!(message, PanelMessage::ToolCall(card) if card.id == "toolu_bash_1")
                       })
                       .expect("placed in the conversation when the turn ended");
    state.push_user_message("again".to_string());
    let next_prompt_at = state.messages
                              .iter()
                              .rposition(|message| matches!(message, PanelMessage::User(_)))
                              .expect("the next prompt");
    assert!(card_at < next_prompt_at,
            "in the errored turn, not the next");
}

/// A background subagent keeps reporting after the turn that launched it
/// ended; its card is known, so its updates still nest.
#[test]
fn a_background_subagent_still_nests_after_the_turn_ends() {
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    state.apply(delegation_start());
    state.apply(turn_end());

    state.apply(subagent_call("toolu_bash_1"));

    assert_eq!(state.tool_call(DELEGATION)
                    .expect("delegation")
                    .subagent
                    .len(),
               1);
}

/// A nested card opens and closes like any other.
#[test]
fn a_nested_card_toggles() {
    let mut state = PanelState::new();
    interleaved(&mut state);
    let before = state.is_collapsed(state.tool_call("toolu_bash_1").expect("nested"));

    state.toggle_tool_call("toolu_bash_1");

    let after = state.is_collapsed(state.tool_call("toolu_bash_1").expect("nested"));
    assert_ne!(before, after);
}

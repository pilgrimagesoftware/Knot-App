//! Tests for text accumulation, turn tracking and the session's lifecycle
//! events.

use knot_acp::PermissionRequest;
use knot_acp::SessionEndCause;
use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;
use serde_json::json;

use super::fixture::*;
use crate::panel_state::*;

#[test]
fn successive_text_deltas_accumulate_into_one_message() {
    let mut state = PanelState::new();

    state.apply(text("Hel"));
    state.apply(text("lo, "));
    state.apply(text("world"));

    assert_eq!(state.messages,
               vec![PanelMessage::Assistant("Hello, world".to_string())]);
}

/// `sent_at` must track `messages` one-to-one - every push goes through
/// `push_message`, which is the only way either `Vec` grows (issue #577).
#[test]
fn sent_at_stays_in_step_with_messages() {
    let mut state = PanelState::new();

    state.push_user_message("hello".to_string());
    state.apply(text("hi there"));
    state.push_error("could not answer".to_string());

    assert_eq!(state.sent_at.len(), state.messages.len());
}

/// Appending to a streaming message in place (`apply(text(..))` twice in a
/// row) must not stamp a new entry - only the message's *first* chunk sets
/// its time.
#[test]
fn appending_to_a_streaming_message_does_not_add_a_second_timestamp() {
    let mut state = PanelState::new();

    state.apply(text("Hel"));
    state.apply(text("lo"));

    assert_eq!(state.sent_at.len(), 1);
}

#[test]
fn user_message_is_recorded_and_does_not_merge_with_assistant_text() {
    let mut state = PanelState::new();

    state.push_user_message("hello".to_string());
    state.apply(text("hi there"));

    assert_eq!(state.messages,
               vec![PanelMessage::User("hello".to_string()),
                    PanelMessage::Assistant("hi there".to_string())]);
}

/// An agent that answers a prompt with an error (an exhausted quota,
/// a dead transport) sends no `TurnEnd`, so the failure has to both
/// show in the conversation and release the composer.
#[test]
fn a_failed_prompt_is_recorded_as_an_error_and_ends_the_turn() {
    let mut state = PanelState::new();

    state.push_user_message("hello".to_string());
    state.push_error("The agent could not answer: quota exhausted".to_string());

    assert_eq!(
               state.messages,
               vec![
        PanelMessage::User("hello".to_string()),
        PanelMessage::Error("The agent could not answer: quota exhausted".to_string())
    ]
    );
    assert!(!state.turn_active,
            "a failed turn must not keep the composer blocked");
}

#[test]
fn permission_request_is_pending_until_resolved() {
    let mut state = PanelState::new();
    let request = PermissionRequest { rpc_id:          json!(1),
                                      tool_call_id:    "tc1".to_string(),
                                      tool_call_title: None,
                                      options:         Vec::new(), };

    state.apply(SessionEvent::PermissionRequest(request.clone()));
    assert_eq!(state.pending_permission().cloned(), Some(request));

    state.resolve_input();
    assert_eq!(state.pending_permission().cloned(), None);
}

#[test]
fn session_end_is_recorded() {
    let mut state = PanelState::new();

    state.apply(SessionEvent::Ended(SessionEndCause::ProcessExited { code: Some(1) }));

    assert_eq!(state.ended,
               Some(SessionEndCause::ProcessExited { code: Some(1) }));
}

#[test]
fn user_message_starts_a_tracked_turn_and_turn_end_ends_it() {
    let mut state = PanelState::new();

    state.push_user_message("hello".to_string());
    assert!(state.turn_active);
    assert!(state.tracking);

    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason:
                                                                  knot_acp::StopReason::EndTurn, }));
    assert!(!state.turn_active);
}

#[test]
fn scrolling_away_clears_tracking_and_a_new_turn_resets_it() {
    let mut state = PanelState::new();
    state.push_user_message("hello".to_string());

    state.clear_tracking();
    assert!(!state.tracking);

    state.push_user_message("again".to_string());
    assert!(state.tracking);
}

#[test]
fn toggle_tracking_flips_the_flag() {
    let mut state = PanelState::new();
    state.push_user_message("hello".to_string());

    state.toggle_tracking();
    assert!(!state.tracking);
    state.toggle_tracking();
    assert!(state.tracking);
}

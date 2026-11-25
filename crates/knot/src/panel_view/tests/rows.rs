//! Tests for the virtualized list's row mapping, row count sync and tail
//! following.

use gpui_kit::ListState;
use gpui_kit::px;

use super::fixture::*;
use crate::panel_state::PanelMessage;
use crate::panel_state::PanelState;
use crate::panel_view::*;

/// The list's item count must cover the trailing permission and ended
/// cards, or tail-following would stop short of them.
#[test]
fn row_count_covers_messages_and_the_trailing_cards() {
    let mut state = PanelState::new();
    assert_eq!(row_count(&state), 0);

    state.messages.push(PanelMessage::User("hi".to_string()));
    assert_eq!(row_count(&state), 1);

    state.pending_input = Some(crate::panel_state::PendingInput::Permission(permission_request()));
    assert_eq!(row_count(&state), 2);

    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: Some(1) });
    assert_eq!(row_count(&state), 3);
}

/// Indices walk the messages in order, then the permission prompt, then
/// the ended banner, and anything past the end resolves to nothing
/// rather than panicking.
#[test]
fn row_at_walks_messages_then_permission_then_ended() {
    let mut state = PanelState::new();
    state.messages.push(PanelMessage::User("a".to_string()));
    state.messages
         .push(PanelMessage::Assistant("b".to_string()));
    state.pending_input = Some(crate::panel_state::PendingInput::Permission(permission_request()));
    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: None });

    assert_eq!(row_at(&state, 0), Some(PanelRow::Message(0)));
    assert_eq!(row_at(&state, 1), Some(PanelRow::Message(1)));
    assert_eq!(row_at(&state, 2), Some(PanelRow::Input));
    assert_eq!(row_at(&state, 3), Some(PanelRow::Ended));
    assert_eq!(row_at(&state, 4), None);
}

/// A question card takes the row a permission prompt would (#430).
#[test]
fn row_at_draws_a_pending_question_where_a_permission_goes() {
    let mut state = PanelState::new();
    state.messages.push(PanelMessage::User("a".to_string()));
    state.pending_input = Some(knot_acp::QuestionRequest { rpc_id:       serde_json::json!(7),
                                session_id:   "sess-1".to_owned(),
                                tool_call_id: None,
                                message:      "Please answer the following questions.".to_owned(),
                                fields:       vec![knot_acp::FormField { key:         "question_0".to_owned(),
                                                                         title:       Some("Color".to_owned()),
                                                                         description: Some("Which color?".to_owned()),
                                                                         required:    false,
                                                                         kind:        knot_acp::FieldKind::Boolean { default: None }, }], }.into());

    assert_eq!(row_count(&state), 2);
    assert_eq!(row_at(&state, 1), Some(PanelRow::Input));
}

/// The ended row is the *last* row, not tied to a fixed index: with no
/// permission pending it follows the messages directly.
#[test]
fn an_ended_row_follows_the_messages_when_no_permission_is_pending() {
    let mut state = PanelState::new();
    state.messages.push(PanelMessage::User("a".to_string()));
    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: Some(0) });

    assert_eq!(row_count(&state), 2);
    assert_eq!(row_at(&state, 1), Some(PanelRow::Ended));
    assert_eq!(row_at(&state, 2), None);
}

/// `sync_row_count` is what keeps a live list's item count in step with
/// the state, growing and shrinking by the delta so rows already on
/// screen are left alone.
#[test]
fn sync_row_count_tracks_messages_and_trailing_rows() {
    let list = ListState::new(0, gpui_kit::ListAlignment::Top, px(LIST_OVERDRAW));
    let mut state = PanelState::new();
    let mut known = 0;

    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (0, 0));

    state.messages.push(PanelMessage::User("a".to_string()));
    state.messages
         .push(PanelMessage::Assistant("b".to_string()));
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (2, 2));

    state.messages
         .push(PanelMessage::Assistant("c".to_string()));
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (3, 3));

    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: None });
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (4, 4));

    state.messages.truncate(1);
    state.ended = None;
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (1, 1));
}

/// The reconcile in `render_panel_pane` mirrors the track toggle onto
/// these two calls: `Tail` while following, `Normal` otherwise. `Normal`
/// is what keeps a toggled-off list put even at the tail, so it must not
/// report as following afterwards.
#[test]
fn follow_mode_reflects_the_toggle() {
    let list = ListState::new(0, gpui_kit::ListAlignment::Top, px(LIST_OVERDRAW));
    assert!(!list.is_following_tail());

    list.set_follow_mode(gpui_kit::FollowMode::Tail);
    assert!(list.is_following_tail());

    list.set_follow_mode(gpui_kit::FollowMode::Normal);
    assert!(!list.is_following_tail());
}

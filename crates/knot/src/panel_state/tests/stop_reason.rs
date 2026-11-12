//! A turn the agent stopped early says why (#22 follow-up): the ACP
//! `stopReason` reaches the conversation, so an agent that announced work
//! and went idle can be told apart from one that ran out of room.
//! Stop reasons from <https://agentclientprotocol.com/protocol/prompt-turn>,
//! "Stop Reasons".

use knot_acp::{SessionEvent, SessionUpdate, StopReason};

use crate::panel_state::{PanelMessage, PanelState, fold::stop_notice};

fn end_turn(reason: StopReason) -> PanelState {
    let mut state = PanelState::new();
    state.push_user_message("go".into());
    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason: reason, }));
    state
}

fn notices(state: &PanelState) -> Vec<&str> {
    state.messages
         .iter()
         .filter_map(|message| match message {
             PanelMessage::Notice(text) => Some(text.as_str()),
             _ => None,
         })
         .collect()
}

#[test]
fn an_early_stop_is_noted_in_the_conversation() {
    for reason in [StopReason::MaxTokens,
                   StopReason::MaxTurnRequests,
                   StopReason::Refusal]
    {
        let state = end_turn(reason.clone());
        assert!(!state.turn_active, "{reason}: the turn still ends");
        assert_eq!(notices(&state).len(), 1, "{reason}: one notice says why");
    }
}

/// Finished, cancelled by the user's own Stop, or failed - which the
/// conversation already shows as the error - adds nothing.
#[test]
fn an_ordinary_end_adds_nothing() {
    for reason in [StopReason::EndTurn,
                   StopReason::Cancelled,
                   StopReason::Error]
    {
        let state = end_turn(reason.clone());
        assert!(!state.turn_active);
        assert!(notices(&state).is_empty(), "{reason}");
    }
}

/// An adapter's own reason, which the spec does not name, is shown as sent.
#[test]
fn an_unknown_reason_is_shown_verbatim() {
    let notice = stop_notice(&StopReason::Other("quota_exhausted".into()))
        .expect("an unknown reason is worth a notice");
    assert!(notice.contains("quota_exhausted"),
            "the reason survives substitution: {notice}");
}

/// The catalogue keys resolve, rather than falling back to the key itself.
#[test]
fn every_notice_resolves_in_the_catalogue() {
    for key in ["panel.turn_stopped_max_tokens",
                "panel.turn_stopped_max_turn_requests",
                "panel.turn_stopped_refusal",
                "panel.turn_stopped_other"]
    {
        assert_ne!(knot_core::l10n::t(key), key, "{key}");
    }
}

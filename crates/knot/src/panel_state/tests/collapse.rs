//! Tests for a tool call card's automatic and user-chosen open/closed state.

use super::fixture::*;
use crate::panel_state::*;

#[test]
fn an_untouched_running_call_is_expanded() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    assert!(!state.is_collapsed(card(&state, 0)),
            "a pending call must stay open");

    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));
    assert!(!state.is_collapsed(card(&state, 0)),
            "a running call's output is what the user is waiting on");
}

#[test]
fn an_untouched_completed_call_is_collapsed() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    assert!(state.is_collapsed(card(&state, 0)));
}

/// The one place this reads past "collapse when they're done": a
/// failure is done, and it is also the card the user opened the
/// conversation to read.
#[test]
fn an_untouched_failed_call_stays_expanded() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("failed"), Vec::new()));

    assert!(!state.is_collapsed(card(&state, 0)));
}

#[test]
fn a_call_opened_while_running_stays_open_when_it_completes() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    // Running calls render open, so a toggle closes one; toggling
    // again is the user explicitly opening it.
    state.toggle_tool_call("tc1");
    state.toggle_tool_call("tc1");
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    assert!(!state.is_collapsed(card(&state, 0)),
            "a call the user opened must not slam shut on completion");
}

#[test]
fn a_call_closed_while_running_stays_closed_when_it_completes() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    state.toggle_tool_call("tc1");
    assert!(state.is_collapsed(card(&state, 0)),
            "a running call can be closed and goes on streaming out of sight");

    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
    assert!(state.is_collapsed(card(&state, 0)));
}

#[test]
fn a_call_closed_while_running_stays_closed_when_it_fails() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    state.toggle_tool_call("tc1");
    state.apply(tool_call_update("tc1", Some("failed"), Vec::new()));

    assert!(state.is_collapsed(card(&state, 0)),
            "the user's choice outranks the automatic expansion of a failure");
}

#[test]
fn opening_one_call_leaves_the_others_alone() {
    let mut state = PanelState::new();
    for id in ["tc1", "tc2"] {
        state.apply(tool_call_start(id, "read"));
        state.apply(tool_call_update(id, Some("completed"), Vec::new()));
    }

    state.toggle_tool_call("tc1");

    assert!(!state.is_collapsed(card(&state, 0)));
    assert!(state.is_collapsed(card(&state, 1)));
}

#[test]
fn toggling_an_unknown_call_records_nothing() {
    let mut state = PanelState::new();

    state.toggle_tool_call("no-such-call");

    assert!(state.tool_call_collapsed.is_empty());
}

/// Nothing about the open/closed choices is persisted: the overrides
/// live in the panel's view state, so a reloaded conversation starts
/// from the automatic behaviour again.
#[test]
fn a_reloaded_conversation_collapses_a_call_the_previous_one_had_open() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
    state.toggle_tool_call("tc1");
    assert!(!state.is_collapsed(card(&state, 0)));

    let mut reloaded = PanelState::new();
    reloaded.apply(tool_call_start("tc1", "execute"));
    reloaded.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    assert!(reloaded.is_collapsed(card(&reloaded, 0)));
}

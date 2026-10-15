//! Permission requests the agent asks for at the same time (#267).
//!
//! An agent running tool calls in parallel asks for several permissions at
//! once, and blocks on each until it is answered. The panel shows one at a
//! time; the rest have to wait their turn rather than replace it.

use knot_acp::{PermissionRequest, SessionEvent, SessionUpdate};
use serde_json::json;

use crate::panel_state::PanelState;

fn request(rpc_id: i64, tool_call_id: &str) -> PermissionRequest {
    PermissionRequest { rpc_id:          json!(rpc_id),
                        tool_call_id:    tool_call_id.to_string(),
                        tool_call_title: None,
                        options:         Vec::new(), }
}

/// The reported defect: the second request replaced the first, which then
/// went unanswered for good, and the agent sat waiting on it with nothing
/// on screen to answer.
#[test]
fn a_second_request_waits_behind_the_first_instead_of_replacing_it() {
    let mut state = PanelState::new();
    let first = request(100, "tc1");
    let second = request(101, "tc2");

    state.apply(SessionEvent::PermissionRequest(first.clone()));
    state.apply(SessionEvent::PermissionRequest(second.clone()));

    assert_eq!(state.pending_permission,
               Some(first),
               "the request on screen stays the one asked first");

    state.resolve_permission();
    assert_eq!(state.pending_permission,
               Some(second),
               "answering it brings up the next, which is still waiting");

    state.resolve_permission();
    assert_eq!(state.pending_permission, None);
}

/// A finished turn - including one Stop cancelled, whose requests the
/// client answered as `cancelled` - has nothing left waiting.
#[test]
fn the_end_of_a_turn_clears_every_request() {
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    state.apply(SessionEvent::PermissionRequest(request(100, "tc1")));
    state.apply(SessionEvent::PermissionRequest(request(101, "tc2")));

    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason: "cancelled".to_string(), }));

    assert!(!state.turn_active);
    assert_eq!(state.pending_permission, None);
    assert!(state.queued_permissions.is_empty());
}

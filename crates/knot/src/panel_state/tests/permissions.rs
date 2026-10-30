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

fn resolved(rpc_id: i64) -> SessionEvent {
    SessionEvent::PermissionResolved { rpc_id: json!(rpc_id), }
}

/// #344: a request answered without the panel - Stop's `cancelled` - stayed
/// pending here, and the agent showed "Awaiting input" with nothing to
/// answer. The client's report takes it off screen, and the next one up.
#[test]
fn a_request_resolved_elsewhere_stops_being_pending() {
    let mut state = PanelState::new();
    let (first, second) = (request(100, "tc1"), request(101, "tc2"));
    state.apply(SessionEvent::PermissionRequest(first));
    state.apply(SessionEvent::PermissionRequest(second.clone()));

    state.apply(resolved(100));

    assert_eq!(state.pending_permission, Some(second));
    state.apply(resolved(101));
    assert_eq!(state.pending_permission, None);
}

/// A queued request resolved first leaves the one on screen alone.
#[test]
fn a_queued_request_resolved_elsewhere_leaves_the_shown_one() {
    let mut state = PanelState::new();
    let (first, second) = (request(100, "tc1"), request(101, "tc2"));
    state.apply(SessionEvent::PermissionRequest(first.clone()));
    state.apply(SessionEvent::PermissionRequest(second));

    state.apply(resolved(101));

    assert_eq!(state.pending_permission, Some(first));
    assert!(state.queued_permissions.is_empty());
}

/// The panel resolves what the user answered at once, and the client's
/// report arrives after. Matched by id, it must not take the next request
/// off screen with it.
#[test]
fn a_report_for_a_request_the_panel_already_resolved_changes_nothing() {
    let mut state = PanelState::new();
    let (first, second) = (request(100, "tc1"), request(101, "tc2"));
    state.apply(SessionEvent::PermissionRequest(first));
    state.apply(SessionEvent::PermissionRequest(second.clone()));
    state.resolve_permission();

    state.apply(resolved(100));

    assert_eq!(state.pending_permission, Some(second));
}

/// The client answers what is still pending as the session ends, but reports
/// it after `Ended`, which the drain never reads past.
#[test]
fn an_ended_session_has_nothing_pending() {
    let mut state = PanelState::new();
    state.apply(SessionEvent::PermissionRequest(request(100, "tc1")));
    state.apply(SessionEvent::PermissionRequest(request(101, "tc2")));

    state.apply(SessionEvent::Ended(knot_acp::SessionEndCause::ProcessExited { code: Some(0) }));

    assert_eq!(state.pending_permission, None);
    assert!(state.queued_permissions.is_empty());
}

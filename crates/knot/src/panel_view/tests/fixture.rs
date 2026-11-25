//! Shared builders for the panel view tests.

use knot_acp::PermissionRequest;

use crate::panel_state::PanelState;

pub(super) fn permission_request() -> PermissionRequest {
    PermissionRequest { rpc_id:          serde_json::json!(1),
                        tool_call_id:    "tc1".to_string(),
                        tool_call_title: None,
                        options:         Vec::new(), }
}

pub(super) fn claude_request() -> PermissionRequest {
    PermissionRequest { options: serde_json::from_value(serde_json::json!([
        { "kind": "allow_always", "name": "Always Allow", "optionId": "allow_always" },
        { "kind": "allow_once", "name": "Allow", "optionId": "allow" },
        { "kind": "reject_once", "name": "Reject", "optionId": "reject" }
    ])).expect("the options deserialize"),
                        ..permission_request() }
}

/// A conversation with a run of tool calls between two pieces of
/// assistant text, for the compact-mode row tests below.
pub(super) fn conversation_with_a_run() -> PanelState {
    fn start(id: &str, kind: &str) -> knot_acp::SessionEvent {
        knot_acp::SessionEvent::Update(knot_acp::SessionUpdate::ToolCallStart {
            tool_call_id: id.to_string(),
            kind:         kind.to_string(),
            title:        String::new(),
            status:       "completed".to_string(),
            content:      Vec::new(),
            raw_input:    None,
            meta:         None,
        })
    }
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    state.apply(start("a", "read"));
    state.apply(start("b", "execute"));
    state.apply(knot_acp::SessionEvent::Update(knot_acp::SessionUpdate::TextDelta {
        text: "done".to_string(),
    }));
    state
}

pub(super) fn tool_call(status: &str) -> crate::panel_state::ToolCallCard {
    crate::panel_state::ToolCallCard { id:        "tc1".to_owned(),
                                       kind:      "execute".to_owned(),
                                       title:     "cargo test".to_owned(),
                                       status:    status.to_owned(),
                                       content:   Vec::new(),
                                       raw_input: None,
                                       meta:      None, }
}

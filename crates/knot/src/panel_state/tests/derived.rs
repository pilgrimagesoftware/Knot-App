//! Tests for what the state reads back out: config options, usage, display
//! names and the last reply.

use knot_acp::ConfigOption;
use knot_acp::PermissionRequest;
use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;
use serde_json::json;

use crate::panel_state::*;

#[test]
fn config_option_update_replaces_the_declared_options() {
    let mut state = PanelState::new();
    let option = ConfigOption { id:            "mode".to_string(),
                                name:          "Mode".to_string(),
                                category:      Some("mode".to_string()),
                                kind:          "select".to_string(),
                                current_value: json!("code"),
                                options:       Vec::new(), };

    state.apply(SessionEvent::Update(SessionUpdate::ConfigOptionUpdate { config_options:
                                                                        vec![option.clone()], }));

    assert_eq!(state.config_options, vec![option]);
}

#[test]
fn usage_update_stores_valid_context_window_size() {
    let mut state = PanelState::new();

    state.apply(SessionEvent::Update(SessionUpdate::Usage { used: 53_000,
                                                            size: 200_000, }));

    assert_eq!(state.context_usage, Some((53_000, 200_000)));
}

fn permission(tool_call_id: &str, title: Option<&str>) -> PermissionRequest {
    PermissionRequest { rpc_id:          json!(1),
                        tool_call_id:    tool_call_id.to_string(),
                        tool_call_title: title.map(str::to_string),
                        options:         Vec::new(), }
}

#[test]
fn display_name_prefers_request_title_then_card_title_kind_then_id() {
    let mut state = PanelState::new();
    state.messages
         .push(PanelMessage::ToolCall(Box::new(ToolCallCard { id:        "tc1".to_string(),
                                                     kind:      "read".to_string(),
                                                     title:
                                                         "Reading configuration file".to_string(),
                                                     status:    "pending".to_string(),
                                                     content:   Vec::new(),
                                                     raw_input: None,
                                                     meta:      None, })));

    assert_eq!(state.display_name(&permission("tc1", Some("Wire title"))),
               "Wire title");
    assert_eq!(state.display_name(&permission("tc1", None)),
               "Reading configuration file");

    state.messages
         .push(PanelMessage::ToolCall(Box::new(ToolCallCard { id:        "tc2".to_string(),
                                                              kind:      "execute".to_string(),
                                                              title:     String::new(),
                                                              status:    "pending".to_string(),
                                                              content:   Vec::new(),
                                                              raw_input: None,
                                                              meta:      None, })));
    assert_eq!(state.display_name(&permission("tc2", None)), "execute");
    assert_eq!(state.display_name(&permission("missing", None)), "missing");
}

fn conversation(messages: Vec<PanelMessage>) -> PanelState {
    let mut state = PanelState::default();
    for message in messages {
        state.push_message(message);
    }
    state
}

/// What Autopilot classifies for a Panel-mode agent: the turn's last text,
/// past any tool calls and notices.
#[test]
fn the_last_reply_is_the_final_text_of_the_latest_turn() {
    let state = conversation(vec![PanelMessage::User("first".into()),
                                  PanelMessage::Assistant("old answer".into()),
                                  PanelMessage::User("second".into()),
                                  PanelMessage::Assistant("looking".into()),
                                  PanelMessage::Assistant("Shall I commit?".into()),
                                  PanelMessage::Notice("note".into())]);
    assert_eq!(state.last_reply(), Some("Shall I commit?"));
}

#[test]
fn a_turn_with_no_text_or_an_error_has_no_reply() {
    let silent = conversation(vec![PanelMessage::User("first".into()),
                                   PanelMessage::Assistant("old answer".into()),
                                   PanelMessage::User("second".into())]);
    assert_eq!(silent.last_reply(),
               None,
               "an earlier turn's answer is not this one's");
    let failed = conversation(vec![PanelMessage::User("go".into()),
                                   PanelMessage::Assistant("partial".into()),
                                   PanelMessage::Error("quota exhausted".into())]);
    assert_eq!(failed.last_reply(), None);
}

#[test]
fn a_notice_is_not_a_turn() {
    let mut state = PanelState::default();
    state.push_notice("Autopilot replied for you".into());
    assert!(!state.turn_active);
    assert_eq!(state.messages.len(), state.sent_at.len());
}

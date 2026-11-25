//! Shared event builders for the panel state tests.

use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;
use knot_acp::ToolCallContent;

use crate::panel_state::PanelMessage;
use crate::panel_state::PanelState;
use crate::panel_state::ToolCallCard;

pub(super) fn text(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TextDelta { text: text.to_string(),
                                                    meta: None, })
}

pub(super) fn tool_call_start(id: &str, kind: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         kind.to_string(),
                                                        title:        String::new(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(),
                                                        raw_input:    None,
                                                        meta:         None, })
}

pub(super) fn tool_call_update(id: &str, status: Option<&str>, content: Vec<ToolCallContent>)
                               -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallUpdate { tool_call_id: id.to_string(),
                                                         status: status.map(str::to_string),
                                                         title: None,
                                                         content,
                                                         raw_input: None,
                                                         meta: None })
}

/// The card at `index`, for the collapse tests below.
pub(super) fn card(state: &PanelState, index: usize) -> &ToolCallCard {
    let PanelMessage::ToolCall(card) = &state.messages[index]
    else {
        panic!("expected a tool call card");
    };
    card
}

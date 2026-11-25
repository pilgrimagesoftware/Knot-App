//! A delegation card's subagent activity (#476): the count line under its
//! header, and - while the card is open - what the subagent did, its tool
//! calls as cards of their own and its prose between them.
//!
//! Drawn inside the card rather than in the conversation, so the main
//! agent's reply reads as one piece however much a subagent does beside it.
//! See `panel_state::subagent` for how the updates get here.

use std::rc::Rc;

use gpui_kit::Div;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::v_flex;
use gpui_kit::div;
use gpui_kit::rgb;

use crate::panel_state::{SubagentItem, ToolCallCard, ToolRunSummary};
use crate::panel_view::style::{MUTED, PanelStyle};
use crate::panel_view::summary_row::summary_text;
use crate::panel_view::tool_call::render_tool_call_card;

/// The line under a delegation's header counting what its subagent ran -
/// the same words as a compact run's line. `None` until it has run anything.
pub(super) fn subagent_count_line(card: &ToolCallCard, style: &PanelStyle) -> Option<Div> {
    let summary = ToolRunSummary::of_cards(card.subagent_tool_calls());
    (summary.calls > 0).then(|| {
                           div().font_family(style.ui_font_family.clone())
                                .text_xs()
                                .text_color(rgb(MUTED))
                                .child(summary_text(summary))
                       })
}

/// What the subagent did, in order, indented under the delegation's header.
/// Each tool call is a full card with its own open/closed state.
pub(super) fn render_subagent_activity(card: &ToolCallCard, style: &PanelStyle,
                                       collapsed_of: &dyn Fn(&ToolCallCard) -> bool,
                                       on_toggle: &Rc<dyn Fn(String)>)
                                       -> Div {
    v_flex().w_full()
            .min_w_0()
            .gap_2()
            .pl_3()
            .border_l_1()
            .border_color(style.border_color)
            .children(card.subagent.iter().map(|item| {
                                              match item {
                                              SubagentItem::ToolCall(child) => {
                                                  render_tool_call_card(child,
                                                                        style,
                                                                        collapsed_of,
                                                                        Rc::clone(on_toggle))
                                                                         .into_any_element()
                                              }
                                              SubagentItem::Text(text) => {
                                                  div().w_full()
                                                       .min_w_0()
                                                       .whitespace_normal()
                                                       .font_family(style.ui_font_family.clone())
                                                       .text_sm()
                                                       .text_color(rgb(MUTED))
                                                       .child(text.clone())
                                                       .into_any_element()
                                              }
                                          }
                                          }))
}

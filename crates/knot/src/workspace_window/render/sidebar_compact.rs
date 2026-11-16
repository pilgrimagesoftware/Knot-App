//! The compact agent row: what a sidebar row shows once it is too narrow for
//! the full one.
//!
//! Contract: `openspec/specs/agent-list-ui/spec.md` - "A narrow sidebar shows
//! a compact layout". Ported from `AgentRowView.compactBody` in the Swift
//! reference.
//!
//! Only the row's *body* lives here. The row's frame - its id, selection
//! background, the dimming of a stopped agent, the companion indent, the
//! click handler and the context menu - stays in [`super::sidebar`] and is
//! identical either way, which is the point: two full row layouts would drift
//! apart the first time one of those changed.

use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::div;
use gpui_kit::px;

use super::status_dot::DotDetail;
use super::status_dot::status_dot;
use crate::workspace_window::key_hints::with_key_hint;

/// What the compact row draws. A struct rather than three positional
/// parameters, and deliberately a subset of [`AgentRow`]: everything the
/// compact layout hides is absent here rather than passed and ignored. The
/// name is not among these - it becomes the row frame's tooltip, which
/// [`super::sidebar`] owns.
pub(crate) struct CompactAgentRow {
    /// The agent, which keys its state dot.
    pub id:            uuid::Uuid,
    /// Already clamped to a single grapheme by the caller.
    pub avatar:        String,
    pub state:         knot_agents::AgentState,
    /// When the agent last went idle, for the dot's tooltip.
    pub idle_since:    Option<std::time::SystemTime>,
    /// Why the agent is waiting on the user, for the dot's tooltip.
    pub input_reason:  Option<String>,
    /// The usage limit that stopped it, for the dot's tooltip.
    pub session_limit: Option<knot_agents::SessionLimit>,
    pub is_shell:      bool,
    /// The row's key hint while ⌘ is held (`key_hints`).
    pub hint:          Option<String>,
}

/// What a compact row's tooltip is built from (#253): with the name and the
/// status lines hidden, the tooltip is where they go.
pub(crate) struct CompactTooltip {
    pub name:          String,
    pub state:         knot_agents::AgentState,
    pub idle_since:    Option<std::time::SystemTime>,
    pub input_reason:  Option<String>,
    pub session_limit: Option<knot_agents::SessionLimit>,
    pub is_shell:      bool,
    /// The status line the full row shows: what the agent says it is doing,
    /// or its terminal title.
    pub status:        String,
    /// The schedule and idle-countdown lines the full row shows (#404).
    pub scheduling:    Vec<String>,
}

/// A compact row's tooltip: the agent's name, its state - worded as its
/// status dot's tooltip words it, so the two cannot disagree - and the
/// status line, each on its own line. A shell agent has no state, as it has
/// no dot; an empty status line is left out.
///
/// Built when the tooltip opens rather than at render, as the dot's is, so
/// "idle since 5m ago" read an hour later still says an hour.
pub(crate) fn compact_tooltip(tooltip: &CompactTooltip) -> String {
    let mut lines = vec![tooltip.name.clone()];
    if !tooltip.is_shell {
        lines.push(crate::app_state::state_tooltip(tooltip.state,
                                                   tooltip.idle_since,
                                                   tooltip.input_reason.as_deref(),
                                                   tooltip.session_limit.as_ref()));
    }
    let status = tooltip.status.trim();
    if !status.is_empty() {
        lines.push(status.to_owned());
    }
    lines.extend(tooltip.scheduling.iter().cloned());
    lines.join("\n")
}

/// The compact row's body: the avatar alone, centred, with the state dot
/// overlaid on its bottom-trailing corner.
///
/// The dot moves onto the avatar because the text block it used to sit
/// beside is gone; it keeps its colour mapping and its "a shell agent has
/// none" rule, so where it sits is the only thing that changes.
pub(crate) fn compact_agent_row_body(row: CompactAgentRow, cx: &gpui_kit::App) -> impl IntoElement {
    let CompactAgentRow { id,
                          avatar,
                          state,
                          idle_since,
                          input_reason,
                          session_limit,
                          is_shell,
                          hint, } = row;
    let dot = (!is_shell).then(|| {
                             status_dot(id,
                                        state,
                                        DotDetail { idle_since,
                                                    input_reason,
                                                    session_limit },
                                        crate::status_palette::status_palette(cx)).absolute()
                                                                                  .bottom_0()
                                                                                  .right_0()
                         });
    let tile = div().relative()
                    .w(px(40.))
                    .h(px(40.))
                    .flex_shrink_0()
                    // As on the full-width row: a legacy avatar carrying more
                    // than one glyph is clamped by the caller, and this keeps
                    // an oversized one inside the tile either way. The dot
                    // sits within these bounds, so it is not what gets cut.
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_2xl()
                    .child(avatar)
                    .children(dot);
    // The hint goes on a box around the tile rather than on the tile, which
    // clips to its bounds and would cut the badge off.
    let framed = with_key_hint(div().relative().child(tile), hint.as_deref(), true, cx);
    h_flex().w_full().justify_center().child(framed)
}

#[cfg(test)]
mod tests;

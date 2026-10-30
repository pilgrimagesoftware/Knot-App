//! The sidebar's state dot, shared by the full and compact rows so its
//! colour, size and tooltip can't drift apart between the two. Where the dot
//! sits is the caller's: beside the text on the full row, over the avatar's
//! corner on the compact one.

use std::time::SystemTime;

use gpui_kit::InteractiveElement;
use gpui_kit::Stateful;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use crate::app_state::state_color;
use crate::app_state::state_tooltip;

/// Agent `id`'s state dot. Hovering it names the state, as the header does
/// beside its own dot, for an idle agent says when its last message was
/// (#582), for one waiting on the user says why (#344), and for one stopped
/// by a usage limit says so (#318). The text is built when the tooltip opens
/// rather than at render, so a "5m ago" read an hour later still says an
/// hour.
pub(super) fn status_dot(id: Uuid, state: knot_agents::AgentState, detail: DotDetail,
                         palette: &crate::status_palette::StatusPalette)
                         -> Stateful<gpui_kit::Div> {
    let DotDetail { idle_since,
                    input_reason,
                    session_limit, } = detail;
    div().id(gpui_kit::ElementId::from(format!("agent-status-dot-{id}")))
         .debug_selector(|| "agent-status-dot".into())
         .flex_shrink_0()
         .w(px(8.))
         .h(px(8.))
         .rounded_full()
         .bg(state_color(state, palette))
         .tooltip(move |window, cx| {
             Tooltip::new(state_tooltip(state,
                                        idle_since,
                                        input_reason.as_deref(),
                                        session_limit.as_ref())).build(window, cx)
         })
}

/// What the dot's tooltip says beyond the state itself.
#[derive(Debug, Clone, Default)]
pub(super) struct DotDetail {
    /// When the agent last went idle.
    pub(super) idle_since:    Option<SystemTime>,
    /// Why it is waiting on the user; `knot_agents::Agent::input_reason`.
    pub(super) input_reason:  Option<String>,
    /// The usage limit that stopped it;
    /// `knot_agents::Agent::active_session_limit`.
    pub(super) session_limit: Option<knot_agents::SessionLimit>,
}

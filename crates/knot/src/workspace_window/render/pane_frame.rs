//! One pane's frame in a multi-pane layout: where it sits, the agent's name
//! above it, the dimming of a pane that is not focused, and click-to-focus
//! (`workspace-layouts`, "Pane contents" and "The focused pane"; design
//! decision 9).
//!
//! A single layout has no frame at all: its pane is drawn bare, with the
//! window's header above it naming the agent.
//!
//! Not here: what the pane shows (`panes::render_agent_pane`) and which pane
//! has focus, which is the layout's (`layout_select`).

use gpui_kit::AnyElement;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::MouseDownEvent;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::black;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::relative;
use uuid::Uuid;

use crate::consts;
use crate::workspace_window::WorkspaceWindow;

/// The height of the name above a pane.
const PANE_HEADER_HEIGHT: f32 = 28.;

/// Where a pane is and whether it has focus.
pub(super) struct PaneSlot {
    pub(super) index:   usize,
    pub(super) agent:   Uuid,
    /// The pane's rectangle as fractions of the layout area.
    pub(super) rect:    Bounds<Pixels>,
    pub(super) focused: bool,
}

impl WorkspaceWindow {
    /// `content` in its pane: positioned, under its agent's name, dimmed
    /// unless focused, and focusing the pane when the mouse goes down in it.
    pub(super) fn pane_frame(&self, slot: &PaneSlot, content: AnyElement, cx: &mut Context<Self>)
                             -> AnyElement {
        let PaneSlot { index,
                       agent,
                       rect,
                       focused, } = *slot;
        let name = self.store
                       .lock()
                       .agent(agent)
                       .map(|agent| agent.name.clone())
                       .unwrap_or_default();
        let header = h_flex().h(px(PANE_HEADER_HEIGHT))
                             .flex_shrink_0()
                             .w_full()
                             .min_w_0()
                             .items_center()
                             .px_3()
                             .border_b_1()
                             .border_color(cx.theme().border)
                             .bg(cx.theme().background)
                             .text_sm()
                             .text_color(if focused {
                                             cx.theme().foreground
                                         }
                                         else {
                                             cx.theme().muted_foreground
                                         })
                             .debug_selector(move || format!("layout-pane-header-{index}"))
                             .child(div().min_w_0().truncate().child(name));
        // Nothing on the overlay listens for anything, so it has no hitbox
        // and the pane's content under it sees every event.
        let overlay = (!focused).then(|| {
                                    div().id(("layout-pane-dim", index))
                               .debug_selector(move || format!("layout-pane-dim-{index}"))
                               .absolute()
                               .top_0()
                               .left_0()
                               .size_full()
                               .bg(black().opacity(consts::UNFOCUSED_PANE_OVERLAY_OPACITY))
                                });
        div().id(("layout-pane", index))
             .debug_selector(move || format!("layout-pane-{index}"))
             .absolute()
             .left(relative(f32::from(rect.origin.x)))
             .top(relative(f32::from(rect.origin.y)))
             .w(relative(f32::from(rect.size.width)))
             .h(relative(f32::from(rect.size.height)))
             .overflow_hidden()
             // Capture phase, so the pane is focused before the event reaches
             // the terminal or the composer under the pointer: one click
             // both focuses the pane and lands in its content.
             .capture_any_mouse_down(cx.listener(move |view, _: &MouseDownEvent, _, cx| {
                                           view.focus_pane(index, cx);
                                       }))
             .child(v_flex().size_full()
                            .child(header)
                            .child(div().flex_1().min_h_0().w_full().child(content)))
             .children(overlay)
             .into_any_element()
    }
}

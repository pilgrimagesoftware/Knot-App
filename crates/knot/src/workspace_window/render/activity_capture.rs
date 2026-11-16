//! Noticing the user in an agent's pane, for idle shutdown (#404): typing,
//! clicking or scrolling there restarts the agent's idle period ("Idle means
//! nothing is happening and nobody is looking").
//!
//! Every listener here is capture-phase and only observes: nothing is
//! consumed, so the composer, the terminal and the conversation list see
//! their events exactly as before. Scrolling needs a canvas because a div's
//! scroll listener runs in the bubble phase, after an inner scroll view may
//! already have stopped the event.

use gpui_kit::{
    AnyElement, Context, DispatchPhase, InteractiveElement, IntoElement, KeyDownEvent,
    MouseDownEvent, ParentElement, ScrollWheelEvent, Styled, canvas, div,
};
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// `pane`, with `id`'s user activity noted as it happens.
    pub(super) fn with_activity_capture(&self, id: Uuid, pane: AnyElement,
                                        cx: &mut Context<Self>)
                                        -> AnyElement {
        let view = cx.entity().downgrade();
        div().size_full()
             .relative()
             .capture_key_down(cx.listener(move |view, _: &KeyDownEvent, _, _| {
                                     view.note_user_activity(id);
                                 }))
             .capture_any_mouse_down(cx.listener(move |view, _: &MouseDownEvent, _, _| {
                                           view.note_user_activity(id);
                                       }))
             .child(pane)
             .child(canvas(|_, _, _| {},
                           move |bounds, (), window, _| {
                               let view = view.clone();
                               window.on_mouse_event(move |event: &ScrollWheelEvent,
                                                           phase,
                                                           _,
                                                           cx| {
                                         if phase == DispatchPhase::Capture
                                            && bounds.contains(&event.position)
                                         {
                                             // A closed window has nothing to note.
                                             let _ = view.update(cx, |view, _| {
                                                             view.note_user_activity(id);
                                                         });
                                         }
                                     });
                           }).absolute()
                             .size_full())
             .into_any_element()
    }
}

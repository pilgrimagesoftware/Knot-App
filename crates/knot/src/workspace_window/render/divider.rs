//! One divider between panes: a 12 px hit area centred on a 1 px line, with
//! the resize cursor for its direction (`workspace-layouts`, "Dividers").
//!
//! The element only starts a drag. Where the pointer goes afterwards, and
//! what the ratio becomes, is the window's business (`workspace_window::
//! layout`): the move and the release have to be heard outside this 12 px
//! strip, which an element's own listeners cannot do.

use gpui_kit::AnyElement;
use gpui_kit::App;
use gpui_kit::CursorStyle;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::MouseButton;
use gpui_kit::MouseDownEvent;
use gpui_kit::ParentElement;
use gpui_kit::SharedString;
use gpui_kit::Styled;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use gpui_kit::px;

use super::layout::DividerId;
use super::layout::DividerRect;
use super::layout::Orientation;
use crate::consts;
use crate::workspace_window::WorkspaceWindow;

/// The element id, and the debug selector tests find the divider by.
pub(in crate::workspace_window) fn divider_name(id: DividerId) -> SharedString {
    match id {
        DividerId::Primary => "layout-divider-primary".into(),
        DividerId::Secondary => "layout-divider-secondary".into(),
    }
}

/// The cursor for a divider that runs `orientation`: a line between columns
/// is dragged sideways, a line between rows up and down.
pub(in crate::workspace_window) fn divider_cursor(orientation: Orientation) -> CursorStyle {
    match orientation {
        Orientation::Vertical => CursorStyle::ResizeLeftRight,
        Orientation::Horizontal => CursorStyle::ResizeUpDown,
    }
}

/// The divider as an element, sized to its hit area. The caller places it at
/// the hit area's origin.
///
/// Takes the window's weak handle rather than a `Context`: it is built while
/// the layout area is being measured, which is outside any `render`.
pub(in crate::workspace_window) fn divider_element(rect: DividerRect,
                                                   view: WeakEntity<WorkspaceWindow>, cx: &App)
                                                   -> AnyElement {
    let divider = rect.divider;
    let name = divider_name(divider.id);
    let line = div().bg(cx.theme().border);
    let line = match divider.orientation {
        Orientation::Vertical => line.w(px(consts::DIVIDER_LINE_WIDTH)).h_full(),
        Orientation::Horizontal => line.h(px(consts::DIVIDER_LINE_WIDTH)).w_full(),
    };
    let debug_name = name.clone();
    div().id(name)
         .debug_selector(move || debug_name.to_string())
         .w(rect.hit.size.width)
         .h(rect.hit.size.height)
         .flex()
         .items_center()
         .justify_center()
         .cursor(divider_cursor(divider.orientation))
         // In front of the panes it overlaps, so that grabbing a divider does
         // not also click into the pane under it and move the focus.
         .block_mouse_except_scroll()
         .on_mouse_down(MouseButton::Left,
                        move |event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
                            // A closed window has no drag to start.
                            let _ = view.update(cx, |view, cx| {
                                            view.begin_divider_drag(divider, event.position, cx);
                                        });
                            window.refresh();
                            cx.stop_propagation();
                        })
         .child(line)
         .into_any_element()
}

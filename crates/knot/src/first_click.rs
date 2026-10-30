//! A window that is not focused takes the click that focuses it, as a
//! standard macOS window does, rather than also handing it to whatever is
//! under the pointer (#309).
//!
//! gpui's macOS view answers `acceptsFirstMouse:` with YES for every window,
//! so the activating click reached buttons, rows and fields: one click on a
//! background window both raised it and pressed whatever it landed on. gpui
//! marks that click (`MouseDownEvent::first_mouse`), and the guard here stops
//! it in the capture phase - before any element sees it - on every window,
//! by decorating each `Root` from one registered plugin.
//!
//! The one exception is a window's own drawn title bar. gpui-kit's `TitleBar`
//! moves the window from its mouse-down, and an inactive window must still be
//! draggable by its title bar in one gesture. Windows with a native title bar
//! need no exception: AppKit handles that bar before gpui sees the click.

use std::any::TypeId;

use gpui_kit::AnyView;
use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::MouseButton;
use gpui_kit::MouseDownEvent;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::RootPlugin;
use gpui_kit::component::Root;
use gpui_kit::component::TITLE_BAR_HEIGHT;
use gpui_kit::div;
use gpui_kit::px;

use crate::command_center::CommandCenterWindow;
use crate::window_options::WORKSPACE_TITLE_BAR_HEIGHT;
use crate::workspace_manager::WorkspaceManager;
use crate::workspace_window::WorkspaceWindow;

/// Whether `event` is a window's activating click that the window should
/// keep to itself: the first left click into an unfocused window, anywhere
/// below its drawn title bar (`drag_strip` high; zero for a native one).
pub(crate) fn absorbs(event: &MouseDownEvent, drag_strip: Pixels) -> bool {
    event.first_mouse && event.button == MouseButton::Left && event.position.y >= drag_strip
}

/// The height of the title bar `view`'s window draws itself, if it draws
/// one - the strip an activating click may still drag the window from.
///
/// Listed by view type because gpui keeps its window-control hit areas to
/// itself. A window that starts drawing a `TitleBar` belongs here.
fn drag_strip(view: &AnyView) -> Pixels {
    let kind = view.entity_type();
    if kind == TypeId::of::<WorkspaceWindow>() {
        px(WORKSPACE_TITLE_BAR_HEIGHT)
    }
    else if kind == TypeId::of::<WorkspaceManager>()
              || kind == TypeId::of::<CommandCenterWindow>()
    {
        TITLE_BAR_HEIGHT
    }
    else {
        px(0.)
    }
}

/// The plugin that guards every window's root. It draws nothing of its own.
pub(crate) struct FirstClickGuard;

impl Render for FirstClickGuard {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::Empty
    }
}

impl RootPlugin for FirstClickGuard {
    fn decorate(&self, surface: gpui_kit::AnyElement, root: &Root, _: &mut Window, _: &mut App)
                -> impl IntoElement {
        let strip = drag_strip(root.view());
        div().size_full()
             .capture_any_mouse_down(move |event, _, cx| {
                 if absorbs(event, strip) {
                     cx.stop_propagation();
                 }
             })
             .child(surface)
    }
}

/// Guards every window opened from here on. After `gpui_kit::init`, which
/// registers gpui-kit's own root plugins.
pub(crate) fn install(cx: &mut App) {
    Root::register_plugin::<FirstClickGuard>(cx, |_, _| FirstClickGuard);
}

#[cfg(test)]
mod tests;

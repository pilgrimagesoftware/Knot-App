//! A context-menu line with a Shift alternate (#475): it reads as its
//! primary, reads as its alternate while Shift is held, and runs whichever
//! it reads as.
//!
//! Shared by the agent row's menu and the sidebar's background menu. What
//! keeps the line honest is that the title and the handler both read
//! `window.modifiers()`, so the title on screen is always what a click,
//! Return or Shift-Return runs. The handler must not read the `ClickEvent`:
//! a mouse click inside a popup reports `shift == false` while Shift is
//! held, and keyboard confirmation hands over `ClickEvent::default()`.
//!
//! An open popup does not redraw on a modifier change by itself. The
//! workspace window's root refreshes the window on every Shift edge
//! ([`crate::workspace_window::WorkspaceWindow::shift_changed`]), which is
//! what makes this line's title follow the key.

use gpui_kit::App;
use gpui_kit::InteractiveElement;
use gpui_kit::KeyBinding;
use gpui_kit::ParentElement;
use gpui_kit::SharedString;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::dialog::Confirm;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::div;

/// The key context gpui-component gives every popup menu.
const POPUP_MENU_CONTEXT: &str = "PopupMenu";

/// The debug selector of a line currently reading `title`, so a test can
/// tell which title an open menu drew.
pub(in crate::workspace_window) fn line_selector(title: &str) -> String {
    format!("menu-line-{title}")
}

/// Shift-Return confirms the highlighted menu line, as Return does.
///
/// gpui-component binds only `enter`, so without this a Shift-Return in a
/// menu matches nothing and the alternate is out of the keyboard's reach.
/// The popup context is the narrowest there is, so this applies to every
/// popup; in one with no alternates it confirms exactly as Return does,
/// which is harmless.
pub(crate) fn alternate_line_key_bindings() -> Vec<KeyBinding> {
    vec![KeyBinding::new("shift-enter",
                         Confirm { secondary: false },
                         Some(POPUP_MENU_CONTEXT))]
}

/// A line reading `primary`, or `alternate` while Shift is held, with a
/// muted Shift marker in the trailing column whether or not it is. `run` is
/// told whether Shift was held when the line was chosen.
pub(in crate::workspace_window) fn alternate_line(primary: String, alternate: String,
                                                  run: impl Fn(bool, &mut Window, &mut App) + 'static)
                                                  -> PopupMenuItem {
    let primary = SharedString::from(primary);
    let alternate = SharedString::from(alternate);
    let marker = SharedString::from(knot_core::l10n::t("menu.agent.shift_marker"));
    PopupMenuItem::element(move |window, app| {
        let title = if window.modifiers().shift {
            alternate.clone()
        }
        else {
            primary.clone()
        };
        let selector = line_selector(&title);
        h_flex().w_full()
                .gap_3()
                .items_center()
                .justify_between()
                .debug_selector(move || selector)
                .child(title)
                .child(div().text_color(app.theme().muted_foreground)
                            .child(marker.clone()))
    }).on_click(move |_, window, app| {
          let shift = window.modifiers().shift;
          run(shift, window, app);
      })
}

impl crate::workspace_window::WorkspaceWindow {
    /// Redraws the window when Shift goes down or up, so an open menu's
    /// alternate lines change title with it. Every other modifier change
    /// is left alone: typing a capital letter costs one frame per Shift
    /// edge, not one per keystroke.
    pub(in crate::workspace_window) fn shift_changed(&mut self,
                                                     event: &gpui_kit::ModifiersChangedEvent,
                                                     window: &mut Window) {
        let shift = event.modifiers.shift;
        if shift != self.shift_held {
            self.shift_held = shift;
            window.refresh();
        }
    }
}

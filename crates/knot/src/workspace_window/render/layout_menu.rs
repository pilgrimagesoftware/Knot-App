//! The Layout menu in the content header: which layout mode the window is
//! in, and the choice of another (`workspace-layouts`, "The Layout menu").
//!
//! The menu is offered while the workspace has two or more agents and the
//! agent view is showing. It lists Single, Split Vertical and Split
//! Horizontal always, Three Panes from three agents and Four Panes from four,
//! so no mode is offered that would leave a pane empty. The current mode is
//! checked.
//!
//! Not here: what choosing a mode does to the layout and to the agents it
//! places (`layout_select::set_layout_mode`).

use gpui_kit::AnyElement;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::div;
use knot_core::LayoutMode;

use crate::workspace_window::WorkspaceWindow;

/// The fewest agents a workspace needs for the Layout menu to be offered.
const MENU_MIN_AGENTS: usize = 2;

/// One line of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace_window) struct LayoutChoice {
    pub(in crate::workspace_window) mode:    LayoutMode,
    pub(in crate::workspace_window) checked: bool,
}

impl LayoutChoice {
    /// The catalogue key of this line's label.
    pub(in crate::workspace_window) const fn label_key(&self) -> &'static str {
        label_key(self.mode)
    }
}

/// The catalogue key naming `mode`. Exhaustive, so a new mode fails to
/// compile until it has a label.
pub(in crate::workspace_window) const fn label_key(mode: LayoutMode) -> &'static str {
    match mode {
        LayoutMode::Single => "layout.single",
        LayoutMode::SplitVertical => "layout.split_vertical",
        LayoutMode::SplitHorizontal => "layout.split_horizontal",
        LayoutMode::ThreePane => "layout.three_panes",
        LayoutMode::GridFourPane => "layout.four_panes",
    }
}

/// The lines the menu lists for a workspace of `agents` agents in `current`,
/// in menu order; empty when the menu is not offered.
pub(in crate::workspace_window) fn layout_choices(agents: usize, current: LayoutMode)
                                                  -> Vec<LayoutChoice> {
    if agents < MENU_MIN_AGENTS {
        return Vec::new();
    }
    LayoutMode::ALL.iter()
                   .copied()
                   // A mode that shows more panes than there are agents would
                   // leave one empty.
                   .filter(|mode| mode.pane_count() <= agents)
                   .map(|mode| LayoutChoice { mode,
                                              checked: mode == current })
                   .collect()
}

impl WorkspaceWindow {
    /// The Layout menu's button, or `None` while the menu is not offered.
    ///
    /// The caller draws it only in the agent view; the agent count is read
    /// here.
    pub(in crate::workspace_window) fn layout_menu(&self, cx: &mut Context<Self>)
                                                   -> Option<AnyElement> {
        let agents = self.workspace_agent_ids().len();
        let current = self.store
                          .lock()
                          .workspace_ui(self.workspace_id)
                          .layout_mode;
        let choices = layout_choices(agents, current);
        if choices.is_empty() {
            return None;
        }
        let view = cx.entity();
        let button =
            Button::new("layout-menu").label(knot_core::l10n::t("layout.menu"))
                                      .dropdown_caret(true)
                                      .ghost()
                                      .small()
                                      .dropdown_menu(move |mut menu, _, _| {
                                          for choice in &choices {
                                              let view = view.clone();
                                              let mode = choice.mode;
                                              menu = menu.item(PopupMenuItem::new(knot_core::l10n::t(choice.label_key()))
                                                  .checked(choice.checked)
                                                  .on_click(move |_, _, app| {
                                                      view.update(app, |view, cx| {
                                                          view.set_layout_mode(mode, cx);
                                                          cx.notify();
                                                      });
                                                  }));
                                          }
                                          menu
                                      });
        Some(div().debug_selector(|| "layout-menu".to_string())
                  .child(button)
                  .into_any_element())
    }
}

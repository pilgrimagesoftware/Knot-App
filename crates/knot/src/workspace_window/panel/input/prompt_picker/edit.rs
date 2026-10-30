//! Adding and editing prompts from the Insert Prompt list (#317, #321): a
//! "+" beside the search field and a "New Prompt…" row at the foot of the
//! list open a blank prompt editor, and a pencil on each row opens that
//! prompt with the caret in its text.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "Adding a prompt from
//! the Insert Prompt list" and "Editing a prompt from the Insert Prompt
//! list".
//!
//! Each closes the list first. The editor is a window of its own, so focus
//! goes there rather than back to the composer, and the composer is left as
//! it was.

use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::px;
use knot_core::Prompt;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Closes the list and opens the prompt editor on `prompt`, or on a
    /// blank prompt for `None`.
    pub(in crate::workspace_window) fn edit_from_prompt_picker(&mut self,
                                                               prompt: Option<Prompt>,
                                                               window: &mut Window,
                                                               cx: &mut Context<Self>) {
        self.close_prompt_picker(false, window, cx);
        // Deferred: this runs inside the workspace window's update, and
        // the editor opens a window of its own. No library window stands
        // behind it to refresh on save, so its parent is left empty.
        cx.defer(move |cx| {
              crate::library_window::open_prompt_editor(WeakEntity::new_invalid(), prompt, cx)
          });
    }
}

/// The "+" beside the search field: a new prompt.
pub(super) fn add_button(key: u64, view: &Entity<WorkspaceWindow>) -> Button {
    let view = view.clone();
    Button::new(("panel-prompt-picker-add", key))
        .icon(IconName::Plus)
        .tooltip(knot_core::l10n::t("panel.prompt_picker.add"))
        .ghost()
        .small()
        .debug_selector(|| "panel-prompt-picker-add".into())
        .on_click(move |_, window, app| {
            view.update(app, |view, cx| view.edit_from_prompt_picker(None, window, cx));
        })
}

/// A row's pencil: that prompt, in the editor. It sits inside the row,
/// whose own click inserts the prompt. The row's handler runs after this
/// one and would find the list already closed, but the click is stopped
/// here rather than left to rely on that order.
pub(super) fn edit_button(key: u64, index: usize, prompt: Prompt, view: &Entity<WorkspaceWindow>)
                          -> Button {
    let view = view.clone();
    Button::new(("panel-prompt-picker-edit", key.wrapping_add(index as u64)))
        .icon(Icon::default().path("icons/pencil.svg"))
        .tooltip(knot_core::l10n::t("panel.prompt_picker.edit"))
        .ghost()
        .xsmall()
        .debug_selector(move || format!("panel-prompt-picker-edit-{index}"))
        .on_click(move |_, window, app| {
            app.stop_propagation();
            let prompt = prompt.clone();
            view.update(app, |view, cx| view.edit_from_prompt_picker(Some(prompt), window, cx));
        })
}

/// The "New Prompt…" row at the foot of the list, set off from the prompts
/// above it by a rule.
pub(super) fn new_prompt_row(key: u64, view: &Entity<WorkspaceWindow>, cx: &gpui_kit::App)
                             -> gpui_kit::Stateful<gpui_kit::Div> {
    let view = view.clone();
    h_flex().id(("panel-prompt-picker-new", key))
            .debug_selector(|| "panel-prompt-picker-new".into())
            .w_full()
            .gap_2()
            .px_2()
            .py_1()
            .border_t_1()
            .border_color(cx.theme().border)
            .rounded(px(4.))
            .cursor_pointer()
            .hover(|row| row.bg(cx.theme().muted))
            .child(Icon::new(IconName::Plus).small())
            .child(knot_core::l10n::t("panel.prompt_picker.new_prompt"))
            .on_click(move |_, window, app| {
                view.update(app, |view, cx| {
                        view.edit_from_prompt_picker(None, window, cx)
                    });
            })
}

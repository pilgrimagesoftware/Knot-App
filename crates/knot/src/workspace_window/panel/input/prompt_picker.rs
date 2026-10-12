//! The composer's Insert Prompt picker: a book button in the control bar
//! whose popover searches the prompt library and inserts the chosen prompt
//! at the caret, or over the selection.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "The composer's Insert
//! Prompt list" and "Choosing a prompt from the Insert Prompt list", and
//! `openspec/specs/keybindings/spec.md`, "Inserting a prompt by shortcut".
//!
//! Modeled on [`super::config_select`]'s searchable list, but a hand-rolled
//! popover like the permission selector's: the kit's `SelectState` keeps its
//! open flag private, and the Insert Prompt shortcut has to open this from
//! an action. Insertion goes through `expand_library_prompt`, the same path
//! the `/` lookup takes, so variables and the clipboard expand alike.
//!
//! The composer's selection is captured when the picker opens. Focus moves
//! to the search field while it is open, and the range it held is what the
//! chosen prompt replaces - an empty one at a bare caret.

use std::ops::Range;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Focusable;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::RenderOnce;
use gpui_kit::ScrollHandle;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Enter;
use gpui_kit::component::input::Escape;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::input::MoveDown;
use gpui_kit::component::input::MoveUp;
use gpui_kit::component::popover::Popover;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;
use knot_core::Prompt;
use uuid::Uuid;

use crate::app_support::single_line;
use crate::keymap::Resolved;
use crate::keymap::Shortcut;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::element_key;

/// How many prompts the list shows before it scrolls.
const PICKER_MAX_VISIBLE: usize = 8;

/// The open picker. At most one per window: it belongs to the composer it
/// was opened from, and only the selected agent's composer is on screen.
pub(in crate::workspace_window) struct PromptPicker {
    agent:           Uuid,
    /// The composer's selection when the picker opened, which the chosen
    /// prompt replaces; empty at a bare caret.
    target:          Range<usize>,
    search:          Entity<InputState>,
    /// Index into the *filtered* list, clamped on use.
    selected:        usize,
    scroll:          ScrollHandle,
    _search_changes: Subscription,
}

/// What the list shows for the current query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace_window) enum PickerRows {
    /// The library holds no prompts at all; the one row opens the Prompts
    /// window.
    EmptyLibrary,
    /// The prompts whose name or text matches, in library order. Empty when
    /// the query matches none.
    Prompts(Vec<Prompt>),
}

/// The prompts whose name or text contains `query`, ignoring case, in
/// library order - or [`PickerRows::EmptyLibrary`] when there are none to
/// filter.
pub(in crate::workspace_window) fn picker_rows(library: &[Prompt], query: &str) -> PickerRows {
    if library.is_empty() {
        return PickerRows::EmptyLibrary;
    }
    let query = query.trim().to_lowercase();
    PickerRows::Prompts(library.iter()
                               .filter(|prompt| {
                                   query.is_empty()
                                   || prompt.name.to_lowercase().contains(&query)
                                   || prompt.text.to_lowercase().contains(&query)
                               })
                               .cloned()
                               .collect())
}

impl WorkspaceWindow {
    /// Whether `id`'s composer has the picker open.
    pub(in crate::workspace_window) fn prompt_picker_open(&self, id: Uuid) -> bool {
        self.prompt_picker
            .as_ref()
            .is_some_and(|picker| picker.agent == id)
    }

    /// Opens the picker for `id`'s composer, remembering its selection and
    /// moving focus to the search field. Nothing for an agent with no
    /// composer, which a shell companion is.
    pub(in crate::workspace_window) fn open_prompt_picker(&mut self, id: Uuid,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>) {
        if self.prompt_picker_open(id) || self.is_shell_agent(id) {
            return;
        }
        let Some(input) = self.panel_prompt_inputs.get(&id)
        else {
            return;
        };
        let target = input.read(cx).selected_range();
        let search = cx.new(|cx| {
                           InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("panel.prompt_picker.search_placeholder"))
                       });
        let subscription = cx.subscribe_in(&search, window, |view: &mut Self, _, event, _, cx| {
                                 if matches!(event, InputEvent::Change)
                                    && let Some(picker) = view.prompt_picker.as_mut()
                                 {
                                     picker.selected = 0;
                                     cx.notify();
                                 }
                             });
        search.update(cx, |state, cx| state.focus(window, cx));
        self.prompt_picker = Some(PromptPicker { agent: id,
                                                 target,
                                                 search,
                                                 selected: 0,
                                                 scroll: ScrollHandle::new(),
                                                 _search_changes: subscription });
        cx.notify();
    }

    /// What the open picker lists, read from the library as it stands, so
    /// an edit made in the Prompts window shows without reopening.
    pub(in crate::workspace_window) fn prompt_picker_rows(&self, cx: &gpui_kit::App)
                                                          -> Option<PickerRows> {
        let picker = self.prompt_picker.as_ref()?;
        let query = picker.search.read(cx).value();
        Some(picker_rows(&crate::settings_global::read(cx).prompts, &query))
    }

    /// Moves the selection by `delta` rows, stopping at either end.
    fn move_prompt_picker(&mut self, delta: isize, cx: &mut Context<Self>) {
        let rows = match self.prompt_picker_rows(cx) {
            Some(PickerRows::Prompts(prompts)) => prompts.len(),
            Some(PickerRows::EmptyLibrary) | None => return,
        };
        let Some(picker) = self.prompt_picker.as_mut()
        else {
            return;
        };
        let last = rows.saturating_sub(1);
        picker.selected = match delta {
            delta if delta < 0 => picker.selected.saturating_sub(delta.unsigned_abs()),
            delta => (picker.selected + delta as usize).min(last),
        };
        picker.scroll.scroll_to_item(picker.selected);
        cx.notify();
    }

    /// Acts on row `index`, or the selected row for `None`: inserts that
    /// prompt into the composer the picker was opened from and closes it,
    /// or, for an empty library, opens the Prompts window. A query matching
    /// nothing leaves the picker open.
    pub(in crate::workspace_window) fn choose_prompt_picker(&mut self, index: Option<usize>,
                                                            window: &mut Window,
                                                            cx: &mut Context<Self>) {
        let Some(rows) = self.prompt_picker_rows(cx)
        else {
            return;
        };
        let prompts = match rows {
            PickerRows::EmptyLibrary => {
                self.close_prompt_picker(true, window, cx);
                // Through the window, not `App::dispatch_action`: this runs
                // inside the window's own update, and the app-level call
                // would update it again, which gpui refuses.
                window.dispatch_action(Box::new(crate::app_bootstrap::OpenPrompts), cx);
                return;
            }
            PickerRows::Prompts(prompts) => prompts,
        };
        let Some(picker) = self.prompt_picker.as_ref()
        else {
            return;
        };
        let index = index.unwrap_or(picker.selected)
                         .min(prompts.len().saturating_sub(1));
        let Some(prompt) = prompts.get(index).map(|prompt| prompt.id)
        else {
            return;
        };
        let (agent, target) = (picker.agent, picker.target.clone());
        self.close_prompt_picker(true, window, cx);
        let Some(input) = self.panel_prompt_inputs.get(&agent).cloned()
        else {
            return;
        };
        self.expand_library_prompt(agent, prompt, target, &input, window, cx);
    }

    /// Closes the picker. With `refocus`, focus returns to the composer it
    /// was opened from with its selection as it was - Esc and a choice do
    /// this; a click elsewhere does not, since focus belongs to what was
    /// clicked.
    pub(in crate::workspace_window) fn close_prompt_picker(&mut self, refocus: bool,
                                                           window: &mut Window,
                                                           cx: &mut Context<Self>) {
        let Some(picker) = self.prompt_picker.take()
        else {
            return;
        };
        if refocus && let Some(input) = self.panel_prompt_inputs.get(&picker.agent) {
            input.update(cx, |state, cx| {
                     state.focus(window, cx);
                     state.set_selected_range(picker.target, cx);
                 });
        }
        cx.notify();
    }

    /// Drops `id`'s picker with the rest of its per-agent state.
    pub(in crate::workspace_window) fn forget_prompt_picker(&mut self, id: Uuid) {
        if self.prompt_picker_open(id) {
            self.prompt_picker = None;
        }
    }

    fn is_shell_agent(&self, id: Uuid) -> bool {
        self.store
            .lock()
            .agent(id)
            .is_some_and(|agent| knot_core::agent_type::is_shell(&agent.agent_type))
    }

    /// The Insert Prompt button and its popover, or `None` for a shell
    /// companion, which has no prompt composer.
    pub(in crate::workspace_window) fn render_prompt_picker(&self, id: Uuid,
                                                            cx: &mut Context<Self>)
                                                            -> Option<gpui_kit::AnyElement> {
        if self.is_shell_agent(id) {
            return None;
        }
        let shortcut = Resolved::from_settings(&crate::settings_global::read(cx).keybindings)
            .chord(Shortcut::InsertPrompt)
            .map(|chord| chord.label())
            .unwrap_or_default();
        let tooltip =
            knot_core::l10n::t_with("panel.prompt_picker.tooltip", &[("shortcut", &shortcut)]);
        let trigger = Button::new("panel-insert-prompt").icon(IconName::BookOpen)
                                                        .tooltip(tooltip)
                                                        .ghost()
                                                        .small();
        let entity = cx.entity();
        let mut popover =
            Popover::new(("panel-insert-prompt-popover", element_key(id)))
                .anchor(gpui_kit::Anchor::BottomRight)
                .trigger(trigger)
                .open(self.prompt_picker_open(id))
                .on_open_change(move |open, window, app| {
                    entity.update(app, |view, cx| {
                              if *open {
                                  view.open_prompt_picker(id, window, cx);
                              }
                              else {
                                  view.close_prompt_picker(false, window, cx);
                              }
                          });
                });
        if let Some(picker) = self.prompt_picker
                                  .as_ref()
                                  .filter(|picker| picker.agent == id)
        {
            let content = self.prompt_picker_content(picker, cx);
            popover = popover.track_focus(&picker.search.focus_handle(cx))
                             .content(move |_, _, _| content.clone());
        }
        Some(popover.into_any_element())
    }

    /// The popover's body: the search field, the rows, and the keys that
    /// drive them. Built per window frame from what the picker holds, so the
    /// content closure only clones it.
    fn prompt_picker_content(&self, picker: &PromptPicker, cx: &mut Context<Self>)
                             -> PickerContent {
        PickerContent { agent:    picker.agent,
                        search:   picker.search.clone(),
                        rows:     self.prompt_picker_rows(cx)
                                      .unwrap_or(PickerRows::Prompts(Vec::new())),
                        selected: picker.selected,
                        scroll:   picker.scroll.clone(),
                        window:   cx.entity(), }
    }
}

#[cfg(test)]
mod tests;

/// Everything the popover draws, captured from the window's frame.
#[derive(Clone, IntoElement)]
struct PickerContent {
    agent:    Uuid,
    search:   Entity<InputState>,
    rows:     PickerRows,
    selected: usize,
    scroll:   ScrollHandle,
    window:   Entity<WorkspaceWindow>,
}

impl RenderOnce for PickerContent {
    fn render(self, _: &mut Window, cx: &mut gpui_kit::App) -> impl IntoElement {
        let PickerContent { agent,
                            search,
                            rows,
                            selected,
                            scroll,
                            window: view, } = self;
        let key = element_key(agent);
        let muted = cx.theme().muted_foreground;
        let highlight = (cx.theme().accent, cx.theme().accent_foreground);
        let row =
            |id, selected: bool, index| picker_row(id, selected.then_some(highlight), &view, index);
        let list: Vec<gpui_kit::AnyElement> = match rows {
            PickerRows::EmptyLibrary => {
                vec![row(("panel-prompt-picker-empty", key), true, None)
                         .child(knot_core::l10n::t("panel.prompt_picker.empty_library"))
                         .into_any_element()]
            }
            PickerRows::Prompts(prompts) if prompts.is_empty() => {
                vec![div().px_2()
                          .py_1()
                          .text_color(muted)
                          .child(knot_core::l10n::t("panel.selector.no_matches"))
                          .into_any_element()]
            }
            PickerRows::Prompts(prompts) => {
                prompts.iter()
                       .enumerate()
                       .map(|(index, prompt)| {
                           row(("panel-prompt-picker-row", key.wrapping_add(index as u64)),
                               index == selected,
                               Some(index)).child(div().flex_shrink_0().child(prompt.name.clone()))
                                           .child(div().flex_1()
                                                       .min_w_0()
                                                       .overflow_hidden()
                                                       .text_ellipsis()
                                                       .whitespace_nowrap()
                                                       .when(index != selected, |el| {
                                                           el.text_color(muted)
                                                       })
                                                       .child(single_line(&prompt.text)))
                                           .into_any_element()
                       })
                       .collect()
            }
        };
        v_flex().id(("panel-prompt-picker", key))
                .w(px(360.))
                .gap_1()
                .capture_action::<MoveUp>(move_handler(&view, -1))
                .capture_action::<MoveDown>(move_handler(&view, 1))
                .capture_action::<Enter>({
                    let view = view.clone();
                    move |_, window, app| {
                        view.update(app, |view, cx| {
                                view.choose_prompt_picker(None, window, cx);
                                cx.stop_propagation();
                            });
                    }
                })
                .capture_action::<Escape>(move |_, window, app| {
                    view.update(app, |view, cx| {
                            view.close_prompt_picker(true, window, cx);
                            cx.stop_propagation();
                        });
                })
                .child(Input::new(&search).small())
                .child(crate::capped_scroll::capped_scroll_list(("panel-prompt-picker-list", key),
                                                                px(PICKER_MAX_VISIBLE as f32
                                                                   * 28.),
                                                                v_flex().w_full()
                                                                        .gap_0p5()
                                                                        .children(list))
                       .track(&scroll))
    }
}

/// A capture handler moving the selection by `delta`, for Up or Down.
fn move_handler<A: 'static>(view: &Entity<WorkspaceWindow>, delta: isize)
                            -> impl Fn(&A, &mut Window, &mut gpui_kit::App) + 'static {
    let view = view.clone();
    move |_, _, app| {
        view.update(app, |view, cx| {
                view.move_prompt_picker(delta, cx);
                cx.stop_propagation();
            });
    }
}

/// One clickable row, tinted with `highlight` (background, text) when
/// selected. `index` is the prompt it inserts, or `None` for the empty
/// library's Open Prompts row.
fn picker_row(id: (&'static str, u64), highlight: Option<(gpui_kit::Hsla, gpui_kit::Hsla)>,
              view: &Entity<WorkspaceWindow>, index: Option<usize>)
              -> gpui_kit::Stateful<gpui_kit::Div> {
    let view = view.clone();
    h_flex().id(id)
            .w_full()
            .min_w_0()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(px(4.))
            .cursor_pointer()
            .when_some(highlight, |row, (bg, fg)| row.bg(bg).text_color(fg))
            .on_click(move |_, window, app| {
                view.update(app, |view, cx| view.choose_prompt_picker(index, window, cx));
            })
}

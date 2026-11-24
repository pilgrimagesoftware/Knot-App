//! The inputs sheet: one field per user input of a library prompt being
//! inserted, Cancel and Insert.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "Inserting a prompt
//! with inputs asks for them" and "At most five inputs are asked for".
//!
//! A window of its own, like `BroadcastSheet`: every dialog in this app
//! that takes more than a yes/no is a window, which is also what makes
//! "starts empty each time" free. The sheet knows nothing about composers
//! or expansion. It reports one [`SheetOutcome`] through `on_close`, exactly
//! once, from whichever way it closes - Insert, Cancel, Esc, the window's
//! close button, or its owner going away - so the caller has one place to
//! tidy up and refocus.
//!
//! Mounted in [`Root`] like every window: Root is what gives the sheet the
//! theme's font and text colour, so without it labels and typed text draw
//! black on the dark background.

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Focusable;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::SharedString;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::WindowHandle;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Escape;
use gpui_kit::component::input::IndentInline;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::input::OutdentInline;
use gpui_kit::div;
use knot_agent_launch::InputValues;
use knot_agent_launch::MAX_PROMPT_INPUTS;
use knot_agent_launch::UserInput;

use crate::app_support::observe_system_appearance;
use crate::window_options::prompt_inputs_window_options;

/// How the sheet closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace_window) enum SheetOutcome {
    /// Insert: the value of every field shown, keyed by label. Inputs past
    /// [`MAX_PROMPT_INPUTS`] are absent, so they take their defaults.
    Insert(InputValues),
    /// Anything else.
    Cancel,
}

/// Called once with how the sheet closed.
type CloseHandler = Box<dyn FnOnce(SheetOutcome, &mut App)>;

/// What the sheet asks for.
pub(in crate::workspace_window) struct SheetRequest {
    /// The prompt's name; the sheet's title.
    pub(in crate::workspace_window) title:  String,
    /// Every input the prompt has, in order. The sheet shows the first
    /// [`MAX_PROMPT_INPUTS`] and notes how many more there are.
    pub(in crate::workspace_window) inputs: Vec<UserInput>,
}

pub(in crate::workspace_window) struct PromptInputsSheet {
    /// One field per input shown, with the label it fills.
    fields:         Vec<(String, Entity<InputState>)>,
    /// Inputs past the cap, which take their defaults.
    more:           usize,
    /// Taken by the first close; `None` once the outcome is reported.
    on_close:       Option<CloseHandler>,
    /// The fields' Return handlers, and whatever [`Self::keep`] adds.
    _subscriptions: Vec<Subscription>,
}

/// Opens the sheet for `request`. `on_close` runs exactly once, when the
/// sheet closes for any reason.
pub(in crate::workspace_window) fn open_prompt_inputs_sheet(request: SheetRequest,
                                                            on_close: impl FnOnce(SheetOutcome,
                                                                   &mut App)
                                                            + 'static,
                                                            cx: &mut App)
                                                            -> Option<WindowHandle<Root>> {
    let options = prompt_inputs_window_options(request.title, cx);
    cx.open_window(options, move |window, cx| {
          // Every window tracks the OS appearance, so a light/dark flip
          // re-resolves the system palette and repaints.
          observe_system_appearance(window);
          let sheet =
              cx.new(|cx| PromptInputsSheet::new(request.inputs, Box::new(on_close), window, cx));
          cx.new(|cx| Root::new(sheet, window, cx))
      })
      .ok()
}

/// The sheet a window from [`open_prompt_inputs_sheet`] shows; `None` once
/// the window has gone.
pub(in crate::workspace_window) fn sheet_in(handle: WindowHandle<Root>, cx: &App)
                                            -> Option<Entity<PromptInputsSheet>> {
    handle.read(cx).ok()?.view().clone().downcast().ok()
}

impl PromptInputsSheet {
    fn new(inputs: Vec<UserInput>, on_close: CloseHandler, window: &mut Window,
           cx: &mut Context<Self>)
           -> Self {
        let more = inputs.len().saturating_sub(MAX_PROMPT_INPUTS);
        let fields: Vec<(String, Entity<InputState>)> =
            inputs.into_iter()
                  .take(MAX_PROMPT_INPUTS)
                  .map(|input| {
                      let default = input.default.unwrap_or_default();
                      let state = cx.new(|cx| InputState::new(window, cx).default_value(default));
                      (input.label, state)
                  })
                  .collect();
        let subscriptions: Vec<Subscription> = fields.iter()
                                                     .enumerate()
                                                     .map(|(index, (_, state))| {
                                                         cx.subscribe_in(state,
                                      window,
                                      move |sheet: &mut Self, _, event, window, cx| {
                                          if let InputEvent::PressEnter { secondary, .. } = event {
                                              sheet.press_enter(index, *secondary, window, cx);
                                          }
                                      })
                                                     })
                                                     .collect();
        // Every way the window can go - including its close button and the
        // owner closing it - releases this entity, so the outcome is
        // reported here if nothing reported it first.
        // `on_release`, not `on_release_in`: by release the window is gone,
        // and a window-bound callback would never run.
        cx.on_release(|sheet, cx| sheet.report(SheetOutcome::Cancel, cx))
          .detach();
        if let Some((_, first)) = fields.first() {
            first.update(cx, |state, cx| {
                     state.focus(window, cx);
                     state.select_all(window, cx);
                 });
        }
        Self { fields,
               more,
               on_close: Some(on_close),
               _subscriptions: subscriptions }
    }

    /// Return moves to the next field and inserts from the last one;
    /// Cmd-Return inserts from any.
    fn press_enter(&mut self, index: usize, secondary: bool, window: &mut Window,
                   cx: &mut Context<Self>) {
        if secondary || index + 1 >= self.fields.len() {
            self.insert(window, cx);
        }
        else {
            self.focus_field(index + 1, window, cx);
        }
    }

    /// Tab and Shift-Tab, wrapping at either end.
    fn step_focus(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.fields.len();
        if count == 0 {
            return;
        }
        let current = self.fields
                          .iter()
                          .position(|(_, state)| state.focus_handle(cx).is_focused(window))
                          .unwrap_or(0);
        let next = if forward {
            (current + 1) % count
        }
        else {
            (current + count - 1) % count
        };
        self.focus_field(next, window, cx);
    }

    fn focus_field(&self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, state)) = self.fields.get(index) {
            state.update(cx, |state, cx| {
                     state.focus(window, cx);
                     state.select_all(window, cx);
                 });
        }
    }

    /// Every shown field's value, keyed by its label.
    fn values(&self, cx: &App) -> InputValues {
        self.fields
            .iter()
            .map(|(label, state)| (label.clone(), state.read(cx).value().to_string()))
            .collect()
    }

    fn insert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let values = self.values(cx);
        self.report(SheetOutcome::Insert(values), cx);
        window.remove_window();
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.report(SheetOutcome::Cancel, cx);
        window.remove_window();
    }

    /// Hands `outcome` to the caller, the first time only.
    fn report(&mut self, outcome: SheetOutcome, cx: &mut App) {
        if let Some(on_close) = self.on_close.take() {
            on_close(outcome, cx);
        }
    }

    /// Holds `subscription` for the sheet's life - how its opener ties the
    /// sheet to the thing that owns it.
    pub(in crate::workspace_window) fn keep(&mut self, subscription: Subscription) {
        self._subscriptions.push(subscription);
    }

    /// The labels of the fields shown, in order - for tests.
    #[cfg(test)]
    pub(in crate::workspace_window) fn labels(&self) -> Vec<String> {
        self.fields.iter().map(|(label, _)| label.clone()).collect()
    }

    /// The fields' states, in order - for tests.
    #[cfg(test)]
    pub(in crate::workspace_window) fn field_states(&self) -> Vec<Entity<InputState>> {
        self.fields.iter().map(|(_, state)| state.clone()).collect()
    }

    /// How many inputs are left to their defaults - for tests.
    #[cfg(test)]
    pub(in crate::workspace_window) fn more(&self) -> usize {
        self.more
    }
}

/// The note under the fields, or `None` when every input is shown.
pub(in crate::workspace_window) fn more_inputs_note(more: usize) -> Option<String> {
    (more > 0).then(|| knot_core::l10n::t_count("prompt_inputs.more_defaulted", more as u64, &[]))
}

impl Render for PromptInputsSheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fields = self.fields.iter().map(|(label, state)| {
                                           v_flex().gap_1()
                                          .child(div().text_sm()
                                                      .child(SharedString::from(label.clone())))
                                          .child(Input::new(state).w_full())
                                       });
        v_flex().size_full()
                .gap_3()
                .px_5()
                .pt_5()
                .pb_6()
                .bg(cx.theme().background)
                // Captured before the focused field sees them: a single-line
                // field would otherwise take Tab as an indent.
                .capture_action(cx.listener(|sheet, _: &IndentInline, window, cx| {
                                    cx.stop_propagation();
                                    sheet.step_focus(true, window, cx);
                                }))
                .capture_action(cx.listener(|sheet, _: &OutdentInline, window, cx| {
                                    cx.stop_propagation();
                                    sheet.step_focus(false, window, cx);
                                }))
                .on_action(cx.listener(|sheet, _: &Escape, window, cx| sheet.cancel(window, cx)))
                .child(v_flex().flex_1().min_h_0().gap_3().children(fields))
                .children(more_inputs_note(self.more).map(|note| {
                                                         div().text_sm()
                                                              .text_color(cx.theme().muted_foreground)
                                                              .child(note)
                                                     }))
                .child(h_flex().flex_shrink_0()
                               .justify_end()
                               .gap_2()
                               .child(Button::new("cancel-prompt-inputs").label(knot_core::l10n::t("prompt_inputs.cancel"))
                                                                         .on_click(cx.listener(|sheet, _, window, cx| {
                                                                             sheet.cancel(window, cx)
                                                                         })))
                               .child(Button::new("insert-prompt-inputs").label(knot_core::l10n::t("prompt_inputs.insert"))
                                                                         .primary()
                                                                         .on_click(cx.listener(|sheet, _, window, cx| {
                                                                             sheet.insert(window, cx)
                                                                         }))))
    }
}

#[cfg(test)]
mod tests;

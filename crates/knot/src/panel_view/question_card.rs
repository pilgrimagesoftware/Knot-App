//! The question card: a form the agent is waiting on, drawn inline where a
//! permission prompt goes (`acp-panel-ui`'s "Question cards", "Submitting or
//! skipping a question card" and "Question card keys").
//!
//! What it shows and what Submit sends come from
//! [`crate::panel_state::QuestionState`]; this module only draws it and turns
//! clicks and keys into [`QuestionAction`]s. The text inputs and the card's
//! focus handle belong to the window ([`QuestionHooks`]), since a row
//! renderer cannot create entities.
//!
//! The card's keys are bound under the `QuestionCard` context, not as fixed
//! app shortcuts: they mean something only inside the card. A binding on an
//! ancestor context also matches while one of the card's text inputs has
//! focus, so each handler acts only when the card itself is focused and
//! otherwise propagates, leaving the key to the input.

use std::rc::Rc;

use gpui_kit::base::{Disableable, h_flex, v_flex};
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ClickEvent, Entity, FocusHandle, KeyBinding, SharedString, Window, actions, div, rgb, rgba,
};
use knot_acp::FieldKind;

use super::callbacks::PanelCallbacks;
use crate::panel_state::{CardKey, FieldValue, QuestionState, Stop};

/// The key context the card's bindings live under.
pub(crate) const QUESTION_CARD_CONTEXT: &str = "QuestionCard";

actions!(question_card,
         [QuestionUp,
          QuestionDown,
          QuestionNextField,
          QuestionPreviousField,
          QuestionToggle,
          QuestionChoose,
          QuestionSubmit,
          QuestionSkip]);

/// The card's keys, scoped to [`QUESTION_CARD_CONTEXT`].
pub(crate) fn question_card_key_bindings() -> Vec<KeyBinding> {
    let context = Some(QUESTION_CARD_CONTEXT);
    vec![KeyBinding::new("up", QuestionUp, context),
         KeyBinding::new("down", QuestionDown, context),
         KeyBinding::new("tab", QuestionNextField, context),
         KeyBinding::new("shift-tab", QuestionPreviousField, context),
         KeyBinding::new("space", QuestionToggle, context),
         KeyBinding::new("enter", QuestionChoose, context),
         KeyBinding::new("secondary-enter", QuestionSubmit, context),
         KeyBinding::new("escape", QuestionSkip, context)]
}

/// What the user did on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuestionAction {
    /// A navigation or pick key, applied by [`QuestionState::key`].
    Key(CardKey),
    /// A click on a choice: pick it (single) or toggle it (multiple).
    Click(Stop),
    /// A click on a boolean field.
    ToggleBoolean(usize),
    Submit,
    /// Escape: leaves a text field for the card, or skips from the card.
    Escape,
    Skip,
}

/// What the window does with a [`QuestionAction`].
pub(crate) type OnQuestionAction = Rc<dyn Fn(QuestionAction, &mut Window, &mut App)>;

/// What the window lends the card for one frame.
#[derive(Clone)]
pub(crate) struct QuestionHooks {
    pub(crate) on_action: OnQuestionAction,
    /// One input per text or number field, by field index.
    pub(crate) inputs:    Rc<Vec<Option<Entity<InputState>>>>,
    /// The card's own focus: where the keyboard is when it is not in an
    /// input.
    pub(crate) focus:     FocusHandle,
}

const ACCENT: u32 = 0x3B82F6;

fn key(key: CardKey) -> QuestionAction {
    QuestionAction::Key(key)
}
/// The keyboard's place on the card is an outline, not a fill: a filled
/// first row read as already picked, and Submit sent nothing (#465). A
/// transparent outline on every row keeps the rows from shifting.
const FOCUS_RING: u32 = 0x3B82F6B3;
const NO_RING: u32 = 0x00000000;

pub(super) fn render_question_card(question: &QuestionState, callbacks: &PanelCallbacks,
                                   window: &Window)
                                   -> impl IntoElement {
    let Some(hooks) = callbacks.question.clone()
    else {
        // A surface without a window behind it (a test, a read-only view)
        // shows the questions without controls.
        return v_flex().child(div().text_sm()
                                   .child(question.request.headline().to_owned()))
                       .into_any_element();
    };
    let card_focused = hooks.focus.is_focused(window);
    let focus = question.focus();
    let message = question.request.message.trim();
    let fields = (0..question.request.fields.len()).filter(|field| !question.is_other(*field))
                                                   .map(|field| {
                                                       render_field(question,
                                                                    field,
                                                                    &hooks,
                                                                    card_focused.then_some(focus)
                                                                                .flatten())
                                                   });
    let on = |action| {
        let on_action = hooks.on_action.clone();
        move |_: &ClickEvent, window: &mut Window, cx: &mut App| on_action(action, window, cx)
    };
    let submit =
        Button::new("question-submit").label(knot_core::l10n::t("panel.question_submit"))
                                      .small()
                                      .primary()
                                      .disabled(!question.can_submit())
                                      .on_click(on(QuestionAction::Submit))
                                      .children(Kbd::global_binding_for_action(&QuestionSubmit,
                                                                               window));
    let skip =
        Button::new("question-skip").label(knot_core::l10n::t("panel.question_skip"))
                                    .small()
                                    .ghost()
                                    .on_click(on(QuestionAction::Skip))
                                    .children(Kbd::global_binding_for_action(&QuestionSkip,
                                                                             window));
    v_flex().id("question-card")
            .track_focus(&hooks.focus)
            .key_context(QUESTION_CARD_CONTEXT)
            .on_action(key_handler::<QuestionUp>(&hooks, key(CardKey::Up), Some(key(CardKey::Up))))
            .on_action(key_handler::<QuestionDown>(&hooks,
                                                   key(CardKey::Down),
                                                   Some(key(CardKey::Down))))
            .on_action(key_handler::<QuestionNextField>(&hooks,
                                                        key(CardKey::Tab),
                                                        Some(key(CardKey::Tab))))
            .on_action(key_handler::<QuestionPreviousField>(&hooks,
                                                            key(CardKey::BackTab),
                                                            Some(key(CardKey::BackTab))))
            .on_action(key_handler::<QuestionToggle>(&hooks, key(CardKey::Space), None))
            .on_action(key_handler::<QuestionChoose>(&hooks, key(CardKey::Return), None))
            .on_action({
                let on_action = hooks.on_action.clone();
                move |_: &QuestionSubmit, window, cx| on_action(QuestionAction::Submit, window, cx)
            })
            .on_action({
                let on_action = hooks.on_action.clone();
                move |_: &QuestionSkip, window, cx| on_action(QuestionAction::Escape, window, cx)
            })
            .gap_3()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(ACCENT))
            .when(!message.is_empty() && question.request.fields.is_empty(),
                  |card| card.child(div().text_sm().child(message.to_owned())))
            .children(fields)
            .child(h_flex().gap_2().child(submit).child(skip))
            .into_any_element()
}

/// A handler for one of the card's own keys. With the card focused it is
/// `action`. From one of the card's inputs - the key reached here because
/// the input let it through - it is `from_input`, or left to the input
/// when that is `None` (Space and Return are typing, not navigation).
fn key_handler<A: gpui_kit::Action>(hooks: &QuestionHooks, action: QuestionAction,
                                    from_input: Option<QuestionAction>)
                                    -> impl Fn(&A, &mut Window, &mut App) + 'static {
    let on_action = hooks.on_action.clone();
    let focus = hooks.focus.clone();
    move |_: &A, window: &mut Window, cx: &mut App| match (focus.is_focused(window), from_input) {
        (true, _) => on_action(action, window, cx),
        (false, Some(from_input)) => on_action(from_input, window, cx),
        (false, None) => cx.propagate(),
    }
}

fn render_field(question: &QuestionState, field: usize, hooks: &QuestionHooks,
                focus: Option<Stop>)
                -> impl IntoElement {
    let form_field = &question.request.fields[field];
    let header = form_field.title
                           .clone()
                           .filter(|title| Some(title) != form_field.description.as_ref());
    let control = match (&form_field.kind, question.value(field)) {
        (FieldKind::Single { choices, .. }, Some(FieldValue::Single(picked))) => {
            v_flex().gap_1()
                    .children(choices.iter().enumerate().map(|(choice, item)| {
                                                            choice_row(Stop { field,
                                                                              choice:
                                                                                  Some(choice) },
                                                                       (*picked == Some(choice),
                                                                        false),
                                                                       (&item.title,
                                                                        item.description
                                                                            .as_deref()),
                                                                       hooks,
                                                                       focus)
                                                        }))
                    .into_any_element()
        }
        (FieldKind::Multi { choices, .. }, Some(FieldValue::Multi(flags))) => {
            v_flex().gap_1()
                    .children(choices.iter().enumerate().map(|(choice, item)| {
                                                            choice_row(Stop { field,
                                                                              choice:
                                                                                  Some(choice) },
                                                                       (flags.get(choice)
                                                                             .copied()
                                                                             .unwrap_or(false),
                                                                        true),
                                                                       (&item.title,
                                                                        item.description
                                                                            .as_deref()),
                                                                       hooks,
                                                                       focus)
                                                        }))
                    .into_any_element()
        }
        (FieldKind::Boolean { .. }, Some(FieldValue::Boolean(flag))) => {
            choice_row(Stop { field,
                              choice: None },
                       (flag.unwrap_or(false), true),
                       (form_field.title.as_deref().unwrap_or(&form_field.key), None),
                       hooks,
                       focus).into_any_element()
        }
        _ => text_input(hooks, field).into_any_element(),
    };
    v_flex().gap_1()
            .children(header.map(|header| div().text_xs().font_weight(gpui_kit::FontWeight::SEMIBOLD).child(header)))
            .children(form_field.description
                                .clone()
                                .map(|description| div().text_sm().child(description)))
            .child(control)
            .children(question.other_of(field).map(|other| {
                                                  let label = question.request.fields[other]
                                                                      .title
                                                                      .clone()
                                                                      .unwrap_or_else(|| knot_core::l10n::t("panel.question_other"));
                                                  h_flex().gap_2()
                                                          .pl_6()
                                                          .items_center()
                                                          .child(div().text_xs().opacity(0.7).child(label))
                                                          .child(div().flex_1().min_w_0().child(text_input(hooks, other)))
                                              }))
}

fn text_input(hooks: &QuestionHooks, field: usize) -> impl IntoElement {
    div().w_full().children(hooks.inputs
                                 .get(field)
                                 .and_then(Option::as_ref)
                                 .map(|input| Input::new(input).small()))
}

/// One choice (or a boolean field): a mark, its title and description.
/// `(on, multiple)`: whether it is picked, and whether it is drawn as a
/// checkbox rather than a radio.
fn choice_row(stop: Stop, (on, multiple): (bool, bool),
              (title, description): (&str, Option<&str>), hooks: &QuestionHooks,
              focus: Option<Stop>)
              -> impl IntoElement {
    let mark = match (multiple, on) {
        (false, true) => "◉",
        (false, false) => "○",
        (true, true) => "☑",
        (true, false) => "☐",
    };
    let on_action = hooks.on_action.clone();
    let action = match stop.choice {
        Some(_) => QuestionAction::Click(stop),
        None => QuestionAction::ToggleBoolean(stop.field),
    };
    h_flex().id(SharedString::from(format!("question-{}-{}",
                                           stop.field,
                                           stop.choice
                                               .unwrap_or(usize::MAX))))
            .gap_2()
            .px_2()
            .py_0p5()
            .rounded_sm()
            .items_start()
            .cursor_pointer()
            .border_1()
            .border_color(rgba(if focus == Some(stop) {
                                   FOCUS_RING
                               }
                               else {
                                   NO_RING
                               }))
            .on_click(move |_: &ClickEvent, window, cx| on_action(action, window, cx))
            .child(div().text_sm().child(mark))
            .child(v_flex().min_w_0()
                           .child(div().text_sm().child(title.to_owned()))
                           .children(description.map(|text| {
                                                    div().text_xs()
                                                         .opacity(0.7)
                                                         .child(text.to_owned())
                                                })))
}

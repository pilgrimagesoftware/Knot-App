//! The window's half of a question card (#430): the text inputs and focus
//! handle the card draws with, and what a click or key on it does.
//!
//! The card's answers live in `PanelState` ([`QuestionState`]), shared with
//! the ACP reader; the entities here cannot be, so they are kept per agent
//! in [`WorkspaceWindow::panel_questions`] and rebuilt when a different
//! request comes on screen. An input writes every change into the state, so
//! what Submit sends never depends on the entities surviving.
//!
//! Focus follows `acp-panel-ui`'s "Question card takes focus": the card
//! takes it when it appears on the visible panel, an input takes it when
//! the keyboard lands on its field, and the composer gets it back when the
//! card goes.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{App, AppContext, Context, Entity, FocusHandle, Focusable, Subscription, Window};
use parking_lot::Mutex;
use serde_json::Value;
use uuid::Uuid;

use crate::panel_session::PanelSessionSlot;
use crate::panel_state::{CardKey, FieldValue, QuestionState, Stop};
use crate::panel_view::{QuestionAction, QuestionHooks};
use crate::workspace_window::WorkspaceWindow;

/// One agent's question card entities, for the request `rpc_id`.
pub(in crate::workspace_window) struct QuestionCardUi {
    rpc_id:         Value,
    inputs:         Rc<Vec<Option<Entity<InputState>>>>,
    focus:          FocusHandle,
    _subscriptions: Vec<Subscription>,
}

type Slot = Arc<Mutex<PanelSessionSlot>>;

/// What the card needs from the state to build its inputs: the request's
/// id, and each text or number field's current text.
fn seed(question: &QuestionState) -> (Value, Vec<Option<String>>) {
    let texts =
        (0..question.request.fields.len()).map(|field| match question.value(field) {
                                              Some(FieldValue::Text(text)) => Some(text.clone()),
                                              _ => None,
                                          })
                                          .collect();
    (question.request.rpc_id.clone(), texts)
}

impl WorkspaceWindow {
    /// Brings `id`'s card entities in step with what is pending and returns
    /// the hooks the card draws with; `None` when no card is on screen.
    /// Called while drawing the visible panel, so a card that appears here
    /// is one the user is looking at, and takes focus.
    pub(in crate::workspace_window) fn question_hooks(&mut self, id: Uuid, slot: &Slot,
                                                      window: &mut Window,
                                                      cx: &mut Context<Self>)
                                                      -> Option<QuestionHooks> {
        let seed = match &*slot.lock() {
            PanelSessionSlot::Ready(handle) => handle.state().lock().pending_question().map(seed),
            _ => None,
        };
        let Some((rpc_id, texts)) = seed
        else {
            self.drop_question_ui(id, window, cx);
            return None;
        };
        if self.panel_questions
               .get(&id)
               .is_none_or(|ui| ui.rpc_id != rpc_id)
        {
            self.panel_questions.remove(&id);
            let ui = self.build_question_ui(id, rpc_id, &texts, window, cx);
            window.focus(&ui.focus, cx);
            self.panel_questions.insert(id, ui);
        }
        let ui = self.panel_questions.get(&id)?;
        let view = cx.entity().downgrade();
        let on_action = move |action: QuestionAction, window: &mut Window, app: &mut App| {
            if let Some(view) = view.upgrade() {
                view.update(app, |view, cx| view.question_action(id, action, window, cx));
            }
        };
        Some(QuestionHooks { on_action: Rc::new(on_action),
                             inputs:    Rc::clone(&ui.inputs),
                             focus:     ui.focus.clone(), })
    }

    /// Forgets `id`'s card entities once no card is pending, handing the
    /// keyboard back to the composer if the card had it.
    fn drop_question_ui(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ui) = self.panel_questions.remove(&id)
        else {
            return;
        };
        if ui.focus.contains_focused(window, cx) {
            let composer = self.panel_prompt_input(id, window, cx);
            composer.update(cx, |state, cx| state.focus(window, cx));
        }
    }

    fn build_question_ui(&mut self, id: Uuid, rpc_id: Value, texts: &[Option<String>],
                         window: &mut Window, cx: &mut Context<Self>)
                         -> QuestionCardUi {
        let mut subscriptions = Vec::new();
        let inputs = texts.iter()
                          .enumerate()
                          .map(|(field, text)| {
                              let text = text.as_ref()?;
                              let input = cx.new(|cx| InputState::new(window, cx).default_value(text.clone()));
                              subscriptions.push(cx.subscribe_in(&input, window, move |view, input, event: &InputEvent, window, cx| {
                                  view.question_input_event(id, field, input, event, window, cx);
                              }));
                              Some(input)
                          })
                          .collect();
        QuestionCardUi { rpc_id,
                         inputs: Rc::new(inputs),
                         focus: cx.focus_handle(),
                         _subscriptions: subscriptions }
    }

    /// An event from the input of `field`: typing is stored, focusing it
    /// moves the card's keyboard position there, Return moves on and
    /// Cmd-Return submits.
    fn question_input_event(&mut self, id: Uuid, field: usize, input: &Entity<InputState>,
                            event: &InputEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            InputEvent::Change => {
                let text = input.read(cx).value().to_string();
                self.edit_question(id, |question| {
                        question.set_text(field, text);
                        true
                    });
            }
            InputEvent::Focus => {
                self.edit_question(id, |question| {
                        question.focus_stop(Stop { field,
                                                   choice: None });
                        true
                    });
            }
            InputEvent::PressEnter { secondary: true, .. } => {
                self.question_action(id, QuestionAction::Submit, window, cx);
                return;
            }
            InputEvent::PressEnter { .. } => {
                self.question_action(id, QuestionAction::Key(CardKey::Tab), window, cx);
                return;
            }
            InputEvent::Blur => return,
        }
        cx.notify();
    }

    fn question_slot(&self, id: Uuid) -> Option<Slot> {
        self.panel_sessions.get(&id).cloned()
    }

    fn edit_question(&self, id: Uuid, edit: impl FnOnce(&mut QuestionState) -> bool) {
        if let Some(slot) = self.question_slot(id)
           && let PanelSessionSlot::Ready(handle) = &*slot.lock()
        {
            handle.edit_question(edit);
        }
    }

    /// What a click or key on `id`'s card does.
    pub(in crate::workspace_window) fn question_action(&mut self, id: Uuid,
                                                       action: QuestionAction,
                                                       window: &mut Window,
                                                       cx: &mut Context<Self>) {
        let answer = |submit: bool| {
            move |view: &Self| {
                if let Some(slot) = view.question_slot(id)
                   && let PanelSessionSlot::Ready(handle) = &*slot.lock()
                {
                    handle.answer_question(submit);
                }
            }
        };
        match action {
            QuestionAction::Key(key) => {
                self.edit_question(id, |question| question.key(key));
                self.focus_question_stop(id, window, cx);
            }
            QuestionAction::Click(stop) => {
                self.edit_question(id, |question| {
                        question.focus_stop(stop);
                        if let Some(choice) = stop.choice {
                            match question.value(stop.field) {
                                Some(FieldValue::Multi(_)) => question.toggle(stop.field, choice),
                                _ => question.pick(stop.field, choice),
                            }
                        }
                        true
                    });
                self.focus_question_card(id, window, cx);
            }
            QuestionAction::ToggleBoolean(field) => {
                self.edit_question(id, |question| {
                        question.focus_stop(Stop { field,
                                                   choice: None });
                        question.toggle_boolean(field);
                        true
                    });
                self.focus_question_card(id, window, cx);
            }
            QuestionAction::Submit => answer(true)(self),
            QuestionAction::Skip => answer(false)(self),
            // From an input: back to the card, typed answers kept. From the
            // card itself: skip. One reflexive Escape never throws away
            // what was typed.
            QuestionAction::Escape => {
                let on_card = self.panel_questions
                                  .get(&id)
                                  .is_some_and(|ui| ui.focus.is_focused(window));
                if on_card {
                    answer(false)(self);
                }
                else {
                    self.focus_question_card(id, window, cx);
                }
            }
        }
        cx.notify();
    }

    fn focus_question_card(&self, id: Uuid, window: &mut Window, cx: &mut App) {
        if let Some(ui) = self.panel_questions.get(&id) {
            window.focus(&ui.focus, cx);
        }
    }

    /// After a key: an input takes the keyboard when focus landed on its
    /// field, the card otherwise.
    fn focus_question_stop(&self, id: Uuid, window: &mut Window, cx: &mut App) {
        let stop = self.question_slot(id).and_then(|slot| match &*slot.lock() {
                                             PanelSessionSlot::Ready(handle) => {
                                                 handle.state()
                                                       .lock()
                                                       .pending_question()
                                                       .and_then(QuestionState::focus)
                                             }
                                             _ => None,
                                         });
        let input = stop.filter(|stop| stop.choice.is_none()).and_then(|stop| {
                                                                 self.panel_questions
                                                                     .get(&id)?
                                                                     .inputs
                                                                     .get(stop.field)?
                                                                     .clone()
                                                             });
        match input {
            Some(input) => {
                let handle = input.focus_handle(cx);
                window.focus(&handle, cx);
            }
            None => self.focus_question_card(id, window, cx),
        }
    }
}

#[cfg(test)]
mod tests;

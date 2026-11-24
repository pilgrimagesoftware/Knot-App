//! Sending a prompt to the agent: reading the composer, delivering or
//! queueing, draining the queue, stopping a turn, and answering a pending
//! permission request.
//!
//! The queue's own rules live in [`crate::workspace_window::prompt_queue`].

use std::sync::Arc;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Window;
use uuid::Uuid;

use crate::panel_session;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue;
use crate::workspace_window::prompt_queue::PromptOrigin;
use crate::workspace_window::prompt_queue::QueuedPanelPrompt;

impl WorkspaceWindow {
    /// Resolves the selected agent's pending permission request (if any,
    /// and if it's a Panel-mode agent) and answers it with `decision` -
    /// backs the allow / always-allow / deny keybindings. A decision the
    /// request does not offer - Always Allow where the agent gave no such
    /// option - leaves it pending rather than answering with something else.
    pub(in crate::workspace_window) fn answer_selected_permission(&self,
                                                                  decision: knot_acp::PermissionDecision)
    {
        let Some(id) = self.selected_agent
        else {
            return;
        };
        if self.store.lock().agent(id).map(|agent| agent.view_mode)
           != Some(knot_core::ViewMode::Panel)
        {
            return;
        }
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return;
        };
        let slot = slot.lock();
        let panel_session::PanelSessionSlot::Ready(handle) = &*slot
        else {
            return;
        };
        let request = handle.state().lock().pending_permission().cloned();
        if let Some(request) = request.filter(|request| request.offers(decision)) {
            handle.answer_permission(&request, decision);
        }
    }

    /// Reads and clears `id`'s prompt input, then hands it to
    /// [`Self::deliver_panel_prompt`].
    pub(in crate::workspace_window) fn send_panel_prompt(&mut self, id: Uuid,
                                                         window: &mut Window,
                                                         cx: &mut Context<Self>) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return;
        };
        let raw = input.read(cx).value().to_string();
        // Before the trim, and before anything reaches the session: the
        // trigger is byte 0 of the *raw* buffer, which is what makes a single
        // leading space the escape for a prompt that has to begin with a
        // literal `!`. Trimming first would make ` !hello` executable and
        // leave the user no way to say it.
        if let Some(command) = crate::panel_commands::shell_command(&raw) {
            if !self.run_panel_shell_command(id, command.to_owned(), cx) {
                return;
            }
            cx.update_entity(&input, |state, cx| {
                  state.set_value("", window, cx);
              });
            cx.notify();
            return;
        }
        // `!` with nothing after it is neither a command nor a message. It
        // has to be refused here too and not only by the send control: the
        // Enter chord reaches this directly, and a lone `!` would otherwise
        // trim to a non-empty string and go to the agent as a prompt.
        if crate::panel_commands::has_shell_trigger(&raw) {
            return;
        }

        let mut text = raw.trim().to_string();
        if text.is_empty() {
            return;
        }
        // Attached context has no dedicated ACP content-block support here
        // (`knot-acp`'s `session/prompt` only sends a single text block),
        // so each path rides along as its own line rather than inventing
        // an unverified resource-attachment wire shape.
        let context = self.panel_pending_context.remove(&id).unwrap_or_default();
        for path in &context {
            text.push_str("\n\nAttached: ");
            text.push_str(&path.to_string_lossy());
        }
        // Every `!` command run since the last prompt rides along here, in
        // the order they were submitted - the only path by which a shell
        // result reaches the agent.
        text.push_str(&self.take_panel_shell_context(id));
        if !self.deliver_panel_prompt(id, text, PromptOrigin::User) {
            return;
        }
        cx.update_entity(&input, |state, cx| {
              state.set_value("", window, cx);
          });
        cx.notify();
    }

    /// Sends `text` through `id`'s live ACP session, or queues it when that
    /// session is mid-turn or blocked on a pending permission, per
    /// `acp-panel-ui`'s "blocking further prompt submission until answered"
    /// requirement.
    ///
    /// Split out of [`Self::send_panel_prompt`] so a broadcast reaches an
    /// agent by exactly the path the agent's own composer uses: the
    /// queueing, the recording of the user's message in the conversation
    /// and the error reporting are one implementation rather than two that
    /// can drift.
    ///
    /// `origin` says what produced the prompt. It is recorded on the queued
    /// entry so a prompt Knot sent of its own accord stays distinguishable
    /// from one the user typed, per `mcp-messaging`'s "Inbox nudges
    /// preserve interrupted session work" - the alternative, matching the
    /// nudge's wording, misclassifies a user who pastes it.
    ///
    /// Returns whether `id` has a panel session at all. An agent with none
    /// is skipped, and its composer is left untouched.
    pub(in crate::workspace_window) fn deliver_panel_prompt(&mut self, id: Uuid, text: String,
                                                            origin: PromptOrigin)
                                                            -> bool {
        let follow = origin.follows(self.reader_at_tail(id));
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let session = {
            let guard = slot.lock();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => {
                    let state = handle.state();
                    let state = state.lock();
                    // A session held by its usage limit queues the prompt
                    // instead, to go when the limit lifts (#368).
                    let ready = !state.has_pending_input()
                                && !state.turn_active
                                && !state.held_by_session_limit(std::time::SystemTime::now());
                    drop(state);
                    ready.then(|| {
                             handle.record_prompt(text.clone(), follow);
                             (handle.session(), handle.recorder())
                         })
                }
                _ => None,
            }
        };
        let Some((session, recorder)) = session
        else {
            self.panel_prompt_queues
                .entry(id)
                .or_default()
                .push(QueuedPanelPrompt::new(text, origin));
            return true;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        if let Err(error) = session.prompt(&text).await {
                            // Shown under the prompt it belongs to, and
                            // it ends the turn - an error response is all
                            // the answer this prompt gets, so the
                            // composer must not stay blocked waiting for
                            // a `TurnEnd` that will never arrive.
                            recorder.error(knot_core::l10n::t_with("panel.error_answer",
                                                                   &[("error",
                                                                      &error.to_string())]));
                            eprintln!("failed to send panel prompt: {error}");
                        }
                    });
        true
    }

    pub(in crate::workspace_window) fn stop_panel_prompt(&mut self, id: Uuid,
                                                         cx: &mut Context<Self>) {
        if !self.panel_stopping.insert(id) {
            return;
        }
        let Some(slot) = self.panel_sessions.get(&id).cloned()
        else {
            self.panel_stopping.remove(&id);
            return;
        };
        let session = {
            let guard = slot.lock();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => Some(handle.session()),
                _ => None,
            }
        };
        let Some(session) = session
        else {
            self.panel_stopping.remove(&id);
            return;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        if let Err(error) = session.cancel().await {
                            eprintln!("failed to cancel agent {id}'s turn: {error}");
                        }
                    });
        cx.notify();
    }

    /// Returns whether a queued prompt was just picked up (moved off the
    /// queue into `panel_prompts_in_flight`) - the caller feeds this into
    /// `deliver_waiting_prompts`'s dirty check, since the queue losing a row
    /// changes the frame with nothing else marking it dirty.
    pub(in crate::workspace_window) fn drain_panel_prompt(&mut self, id: Uuid) -> bool {
        let reader_at_tail = self.reader_at_tail(id);
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let mut candidate = || -> Option<_> {
            let guard = slot.lock();
            let panel_session::PanelSessionSlot::Ready(handle) = &*guard
            else {
                return None;
            };
            let state = handle.state();
            let state = state.lock();
            if state.has_pending_input() || state.turn_active {
                return None;
            }
            // Checked every tick, so a held prompt goes on the first one
            // after the limit lifts (#368).
            let held = state.held_by_session_limit(std::time::SystemTime::now());
            drop(state);
            // `turn_active` flips only once the spawned prompt reaches the
            // session, so without this a tick in between would send the
            // next prompt as well.
            if self.panel_prompts_in_flight.contains_key(&id) {
                return None;
            }
            let queue = self.panel_prompt_queues.get_mut(&id)?;
            let prompt = prompt_queue::take_next(queue, held)?;
            handle.record_prompt(prompt.text.clone(), prompt.origin.follows(reader_at_tail));
            let sent = (handle.session(), handle.recorder(), prompt.text.clone(), prompt.id);
            self.panel_prompts_in_flight.insert(id, prompt);
            Some(sent)
        };
        let Some((session, recorder, text, prompt_id)) = candidate()
        else {
            return false;
        };
        let results = Arc::clone(&self.panel_prompt_results);
        self.runtime.spawn(async move {
                        let result = session.prompt(&text)
                                            .await
                                            .map_err(|error| error.to_string());
                        if let Err(error) = &result {
                            recorder.error(knot_core::l10n::t_with("panel.error_answer",
                                                                   &[("error",
                                                                      &error.to_string())]));
                        }
                        {
                            let mut results = results.lock();
                            results.push((id, prompt_id, result));
                        }
                    });
        true
    }
}

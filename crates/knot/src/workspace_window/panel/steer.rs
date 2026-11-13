//! Steer now: handing a queued prompt to the agent's running turn instead of
//! waiting for it to end (#26, `openspec/changes/interruptable-prompts`).
//!
//! Offered only for an agent that advertised `_session/steering` and only
//! while its turn runs. The steer goes out on the runtime; its answer comes
//! back through `panel_steer_results`, which the repaint poll drains, so the
//! row changes on screen as soon as the agent answers.

use std::sync::Arc;

use knot_acp::SteerOutcome;
use uuid::Uuid;

use crate::panel_session;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::{self, SteerResult};

/// One steer's answer: the agent, the queue entry, and what came back.
pub(crate) type PanelSteerResult = (Uuid, Uuid, Result<SteerOutcome, String>);

impl WorkspaceWindow {
    /// Whether `id`'s queued prompts can be steered right now: its session
    /// is connected, its agent advertised steering, a turn is running, and
    /// no permission request is waiting. That last is `acp-panel-ui`'s
    /// "Permission gate is independent of the queue": nothing reaches the
    /// agent while it waits on the user, steered or not.
    pub(in crate::workspace_window) fn can_steer(&self, id: Uuid) -> bool {
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let guard = slot.lock();
        let panel_session::PanelSessionSlot::Ready(handle) = &*guard
        else {
            return false;
        };
        let state = handle.state();
        let state = state.lock();
        handle.session().supports_steering()
        && state.turn_active
        && state.pending_permission.is_none()
    }

    /// Steers queued entry `entry` of agent `id` into its running turn.
    /// Returns whether a steer went out; nothing is sent when the agent
    /// cannot take one or the entry is gone, failed or already on its way.
    pub(in crate::workspace_window) fn steer_queued_prompt(&mut self, id: Uuid, entry: Uuid)
                                                           -> bool {
        if !self.can_steer(id) {
            return false;
        }
        let Some(session) =
            self.panel_sessions
                .get(&id)
                .and_then(|slot| match &*slot.lock() {
                    panel_session::PanelSessionSlot::Ready(handle) => Some(handle.session()),
                    _ => None,
                })
        else {
            return false;
        };
        let Some(text) = self.panel_prompt_queues
                             .get_mut(&id)
                             .and_then(|queue| prompt_queue::begin_steer(queue, entry))
        else {
            return false;
        };
        let results = Arc::clone(&self.panel_steer_results);
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        let result = session.steer(&text)
                                            .await
                                            .map_err(|error| error.to_string());
                        results.lock().push((id, entry, result));
                    });
        true
    }

    /// Settles every steer that has answered, saying whether any did - for
    /// the repaint poll's dirty check, since the answer lands off the main
    /// thread with nothing else to repaint it.
    pub(in crate::workspace_window) fn drain_steer_results(&mut self) -> bool {
        let results = std::mem::take(&mut *self.panel_steer_results.lock());
        let drained = !results.is_empty();
        for (id, entry, result) in results {
            let settled = match &result {
                Ok(SteerOutcome::Injected | SteerOutcome::StartedNewTurn) => SteerResult::Delivered,
                Ok(SteerOutcome::PromptRequired) => SteerResult::Returned,
                Ok(SteerOutcome::Failed) => {
                    eprintln!("agent {id}'s adapter could not steer the prompt");
                    SteerResult::Failed
                }
                Err(error) => {
                    eprintln!("failed to steer agent {id}'s prompt: {error}");
                    SteerResult::Failed
                }
            };
            let Some(queue) = self.panel_prompt_queues.get_mut(&id)
            else {
                continue;
            };
            let Some(text) = prompt_queue::finish_steer(queue, entry, settled)
            else {
                continue;
            };
            if let Some(slot) = self.panel_sessions.get(&id)
               && let panel_session::PanelSessionSlot::Ready(handle) = &*slot.lock()
            {
                handle.record_steered_prompt(text);
            }
        }
        drained
    }
}

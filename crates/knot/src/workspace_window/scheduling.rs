//! What a workspace window answers and does for a scheduled start (#404).
//!
//! Contract: `openspec/changes/session-scheduling` - "A scheduled run never
//! discards the agent's work" and "An agent can be started on a schedule".
//! The app-level ticker (`crate::scheduled_starts`) decides *when*; it asks
//! the window owning an agent's session whether starting now would discard
//! work, and has it start the run, because the sessions, the prompt queue and
//! the pending permission all live here.

use gpui_kit::Context;
use uuid::Uuid;

use crate::panel_session::PanelSessionSlot;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Whether starting `id` in a new conversation now would discard work: a
    /// turn in progress, a pending permission request, a queued prompt, or a
    /// message it read and has not answered (#405).
    pub(crate) fn scheduled_run_busy(&self, id: Uuid) -> bool {
        let mid_turn = self.panel_sessions
                           .get(&id)
                           .is_some_and(|slot| match &*slot.lock() {
                               PanelSessionSlot::Ready(handle) => {
                                   let state = handle.state();
                                   let state = state.lock();
                                   state.turn_active || state.pending_permission.is_some()
                               }
                               _ => false,
                           });
        let queued = self.panel_prompt_queues
                         .get(&id)
                         .is_some_and(|queue| !queue.is_empty());
        mid_turn || queued || {
            let store = self.store.lock();
            let messages = self.messages.lock();
            owes_a_reply(&store, &messages, id)
        }
    }

    /// Starts `id` for a scheduled run: marks it activated and restarts it in
    /// a new conversation, so exactly one fresh session starts and its
    /// startup prompt is queued once. Not `activate_agent`, whose
    /// `start_sessions` would start a session only for the restart to tear it
    /// down again. Both happen in this one main-thread step, so the repaint
    /// poll's start-if-activated rule cannot run between them.
    pub(crate) fn start_scheduled_run(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.store.lock().set_activated(id, true);
        self.restart_agents(&[id], false, cx);
        cx.notify();
    }
}

/// Whether `id` read a message it has not answered, from a sender still in
/// the store: the same test "Knot's own reset waits for open threads" uses.
/// Takes the two stores rather than a window, so the app-level ticker can ask
/// it about an agent whose window is closed.
pub(crate) fn owes_a_reply(store: &knot_agents::AgentStore,
                           messages: &knot_messaging::MessageStore, id: Uuid)
                           -> bool {
    messages.has_unanswered(id, |sender| store.agent(sender).is_some())
}

#[cfg(test)]
mod tests;

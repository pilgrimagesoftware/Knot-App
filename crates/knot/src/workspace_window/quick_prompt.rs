//! What the dashboards ask of a workspace window (#10,
//! `openspec/changes/dashboard-status-and-quick-prompt`): whether an agent
//! has a session to send to, and a quick prompt for it.
//!
//! Both dashboards go through these two calls - the Command Center, which
//! holds no sessions, reaches them through `WindowRegistry::workspace_view` -
//! so neither needs the composer's internals.

use uuid::Uuid;

use crate::panel_session::{PanelPhase, PanelSessionSlot};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

impl WorkspaceWindow {
    /// Whether `id` has a session a prompt can reach: a terminal session, or
    /// a panel session that is connecting or ready. A connecting one counts
    /// because a prompt queues into it and goes once it is ready; a failed
    /// one does not, because nothing drains its queue (#10 review).
    pub(crate) fn has_session(&self, id: Uuid) -> bool {
        if self.sessions.contains_key(&id) {
            return true;
        }
        self.panel_sessions
            .get(&id)
            .is_some_and(|slot| has_live_phase(&slot.lock()))
    }

    /// Sends `text` to `id` as the user's prompt - sent now, or queued
    /// behind a turn, a permission or a usage limit, as the composer's
    /// prompts are. Returns whether it was taken; `false` when the agent has
    /// no session that can take it, so the caller keeps the text.
    pub(crate) fn quick_prompt(&mut self, id: Uuid, text: &str) -> bool {
        if !self.has_session(id) || !self.panel_sessions.contains_key(&id) {
            return false;
        }
        self.deliver_panel_prompt(id, text.to_owned(), PromptOrigin::User)
    }
}

/// Whether a panel slot in this phase can take a prompt.
pub(in crate::workspace_window) fn has_live_phase(slot: &PanelSessionSlot) -> bool {
    match slot.phase() {
        PanelPhase::Connecting(_) | PanelPhase::Ready => true,
        PanelPhase::Failed | PanelPhase::Retired => false,
    }
}

#[cfg(test)]
mod tests;

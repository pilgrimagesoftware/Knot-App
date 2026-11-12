//! What a restart into a new conversation hands the next one (#405): the
//! agent's unanswered briefs, reopened in its inbox, and a note for its first
//! turn naming them and its last status. The note's wording is
//! `knot_agent_launch::handoff_note`; this is where the window gathers what
//! goes in it.

use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Reopens `id`'s unanswered messages - back in its inbox, unread - and
    /// returns the note its next conversation should open with, naming its
    /// last status and who it owed replies to. `None` when it had neither.
    ///
    /// Called for a restart into a new conversation, which keeps nothing of
    /// the old one; a brief read there and not yet answered would otherwise
    /// be lost with it (#405).
    pub(super) fn hand_off(&mut self, id: Uuid) -> Option<String> {
        // The store first, as everywhere both are held: briefs from an agent
        // that is gone can never be answered, so they are not reopened.
        let store = self.store.lock();
        let reopened = self.messages
                           .lock()
                           .reopen_unanswered(id, |sender| store.agent(sender).is_some());
        let status = store.agent(id)
                          .map(|agent| agent.status_text.clone())
                          .unwrap_or_default();
        let threads = reopened.iter()
                              .map(|message| {
                                  knot_agent_launch::OpenThread { from:    store.agent(message.from)
                                                                      .map_or_else(|| {
                                                                          message.from.to_string()
                                                                      },
                                                                      |agent| agent.name.clone()),
                                                        sent_at: rfc3339(message.timestamp), }
                              })
                              .collect::<Vec<_>>();
        knot_agent_launch::handoff_note(&status, &threads)
    }
}

/// `time` as `check-messages` reports a message's `timestamp`, so the
/// handoff note and the inbox name a message the same way.
fn rfc3339(time: std::time::SystemTime) -> String {
    time::OffsetDateTime::from(time).format(&time::format_description::well_known::Rfc3339)
                                    .unwrap_or_default()
}

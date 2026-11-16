//! What a restart into a new conversation hands the next one (#405): the
//! agent's unanswered briefs, reopened in its inbox, and a note for its first
//! turn naming them and its last status. Each sender of those briefs is told,
//! too (#405 item 4). The wording is `knot_agent_launch::handoff_note` and
//! `reset_notice`; this is where the window gathers what goes in them.

use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Reopens `id`'s unanswered messages - back in its inbox, unread - and
    /// returns the note its next conversation should open with, naming its
    /// last status and who it owed replies to. `None` when it had neither.
    ///
    /// Called for a restart into a new conversation, which keeps nothing of
    /// the old one; a brief read there and not yet answered would otherwise
    /// be lost with it (#405). Each sender of a reopened brief gets one
    /// notice saying so, from `id`, which the inbox nudge delivers like any
    /// other message.
    pub(super) fn hand_off(&mut self, id: Uuid) -> Option<String> {
        // The store first, as everywhere both are held: briefs from an agent
        // that is gone can never be answered, so they are not reopened.
        let store = self.store.lock();
        let mut messages = self.messages.lock();
        let reopened = messages.reopen_unanswered(id, |sender| store.agent(sender).is_some());
        let name = store.agent(id)
                        .map_or_else(|| id.to_string(), |agent| agent.name.clone());
        for (sender, sent_at) in by_sender(&reopened) {
            messages.add(knot_messaging::Message::notice(id,
                                                         sender,
                                                         knot_agent_launch::reset_notice(&name,
                                                                                         &sent_at)));
        }
        drop(messages);
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

/// The reopened messages' send times, grouped by sender in the order each
/// sender first appears, so a sender of several briefs gets one notice.
fn by_sender(reopened: &[knot_messaging::Message]) -> Vec<(Uuid, Vec<String>)> {
    let mut grouped: Vec<(Uuid, Vec<String>)> = Vec::new();
    for message in reopened {
        let sent = rfc3339(message.timestamp);
        match grouped.iter_mut()
                     .find(|(sender, _)| *sender == message.from)
        {
            Some((_, times)) => times.push(sent),
            None => grouped.push((message.from, vec![sent])),
        }
    }
    grouped
}

/// `time` as `check-messages` reports a message's `timestamp`, so the
/// handoff note and the inbox name a message the same way.
fn rfc3339(time: std::time::SystemTime) -> String {
    time::OffsetDateTime::from(time).format(&time::format_description::well_known::Rfc3339)
                                    .unwrap_or_default()
}

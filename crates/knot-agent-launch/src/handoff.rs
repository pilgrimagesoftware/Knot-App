//! The note a new conversation opens with when it replaces one that had
//! work in hand (#405).
//!
//! A restart into a new conversation leaves the agent with nothing of the
//! old one: not the status it last reported, and not the briefs it had read
//! and not yet answered. Those briefs are put back in its inbox (see
//! `knot_messaging::MessageStore::reopen_unanswered`); this note is what
//! tells the new conversation that they are there, and what the old one
//! said it was doing. One line, like the registration prompt it follows.

/// A message the old conversation had read and not answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenThread {
    /// Who sent it: an agent's name, or its id when the agent is gone.
    pub from:    String,
    /// When it was sent, as `check-messages` reports a message's
    /// `timestamp`, so the agent can match the two.
    pub sent_at: String,
}

/// The handoff note for a conversation replacing one whose last status was
/// `status` and which left `threads` unanswered, or `None` when there is
/// neither: a conversation that had nothing in hand needs no note.
pub fn handoff_note(status: &str, threads: &[OpenThread]) -> Option<String> {
    let status = status.trim();
    if status.is_empty() && threads.is_empty() {
        return None;
    }
    let mut note =
        "Your previous conversation in this knot was reset, so you start without it.".to_owned();
    if !status.is_empty() {
        note.push_str(&format!(" Your last status was: \"{status}\"."));
    }
    if !threads.is_empty() {
        let senders = threads.iter()
                             .map(|thread| format!("{} (sent {})", thread.from, thread.sent_at))
                             .collect::<Vec<_>>()
                             .join(", ");
        note.push_str(&format!(" You had read and not yet answered messages from {senders}. \
                                They are back in your inbox: call check-messages to read them \
                                in full, check what you already did for them (your branch, your \
                                worktree), and carry on with that work."));
    }
    Some(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread(from: &str) -> OpenThread {
        OpenThread { from:    from.to_owned(),
                     sent_at: "2026-11-12T20:00:00Z".to_owned(), }
    }

    #[test]
    fn nothing_in_hand_needs_no_note() {
        assert_eq!(handoff_note("  ", &[]), None);
    }

    #[test]
    fn the_note_names_the_status_and_every_sender() {
        let note = handoff_note("Fixing #405", &[thread("Knot Master"), thread("Builder 1")])
            .expect("work in hand gets a note");

        assert!(note.contains("Fixing #405"));
        assert!(note.contains("Knot Master (sent 2026-11-12T20:00:00Z)"));
        assert!(note.contains("Builder 1"));
        assert!(note.contains("check-messages"));
        assert!(!note.contains('\n'),
                "one line, like the registration prompt it follows");
    }

    #[test]
    fn a_status_alone_still_gets_a_note() {
        let note = handoff_note("Reviewing #400", &[]).expect("a status is work in hand");
        assert!(note.contains("Reviewing #400"));
        assert!(!note.contains("check-messages"),
                "no threads, nothing to read back");
    }
}

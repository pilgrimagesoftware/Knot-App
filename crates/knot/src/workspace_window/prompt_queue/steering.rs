//! Steering a queued prompt into the running turn (#26): the entry's side
//! of it. The request itself is `knot_acp`'s `session_steer`; the window
//! sends it and brings the answer back here.

use uuid::Uuid;

use super::{QueuedPanelPrompt, position};

/// How a steer ended, for the entry it was steering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SteerResult {
    /// The agent took it into its turn: the entry leaves the queue.
    Delivered,
    /// No turn was running, so it was handed back: the entry goes to the
    /// head and the turn end sends it as the next turn.
    Returned,
    /// The steer failed: the entry goes to the head, marked failed, as a
    /// failed delivery does.
    Failed,
}

/// Marks `id` as being steered and returns its text, or `None` when it is
/// gone, failed, or already being steered - nothing is sent for those.
pub(crate) fn begin_steer(queue: &mut [QueuedPanelPrompt], id: Uuid) -> Option<String> {
    let prompt = queue.iter_mut().find(|prompt| prompt.id == id)?;
    if prompt.failed || prompt.steering {
        return None;
    }
    prompt.steering = true;
    Some(prompt.text.clone())
}

/// Settles the steer of `id` as `result` says, returning the prompt's text
/// when it was delivered, for the conversation to record. `None` when the
/// entry is gone (deleted while the steer was out) or was not being steered.
pub(crate) fn finish_steer(queue: &mut Vec<QueuedPanelPrompt>, id: Uuid, result: SteerResult)
                           -> Option<String> {
    let index = position(queue, id)?;
    if !queue[index].steering {
        return None;
    }
    let mut prompt = queue.remove(index);
    prompt.steering = false;
    match result {
        SteerResult::Delivered => Some(prompt.text),
        SteerResult::Returned => {
            queue.insert(0, prompt);
            None
        }
        SteerResult::Failed => {
            prompt.failed = true;
            queue.insert(0, prompt);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_window::prompt_queue::{PromptOrigin, take_next};

    fn queue_of(texts: &[&str]) -> Vec<QueuedPanelPrompt> {
        texts.iter()
             .map(|text| QueuedPanelPrompt::new((*text).to_owned(), PromptOrigin::User))
             .collect()
    }

    #[test]
    fn a_delivered_steer_leaves_the_queue() {
        let mut queue = queue_of(&["first", "fix"]);
        let id = queue[1].id;
        assert_eq!(begin_steer(&mut queue, id).as_deref(), Some("fix"));

        assert_eq!(finish_steer(&mut queue, id, SteerResult::Delivered).as_deref(),
                   Some("fix"));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].text, "first");
    }

    /// The turn ended before the steer landed: the prompt goes next.
    #[test]
    fn a_returned_steer_goes_back_to_the_head() {
        let mut queue = queue_of(&["first", "fix"]);
        let id = queue[1].id;
        begin_steer(&mut queue, id);

        assert_eq!(finish_steer(&mut queue, id, SteerResult::Returned), None);
        assert_eq!(queue[0].text, "fix");
        assert!(!queue[0].steering && !queue[0].failed);
    }

    #[test]
    fn a_failed_steer_stays_queued_and_marked() {
        let mut queue = queue_of(&["fix"]);
        let id = queue[0].id;
        begin_steer(&mut queue, id);

        finish_steer(&mut queue, id, SteerResult::Failed);
        assert!(queue[0].failed);
        assert!(!queue[0].steering);
    }

    /// While its steer is out, the turn end does not send it as well.
    #[test]
    fn the_turn_end_passes_over_an_entry_being_steered() {
        let mut queue = queue_of(&["fix", "next"]);
        let id = queue[0].id;
        begin_steer(&mut queue, id);

        let taken = take_next(&mut queue, false).expect("the entry behind it goes");
        assert_eq!(taken.text, "next");
        assert_eq!(take_next(&mut queue, false).map(|p| p.text), None);
    }

    #[test]
    fn nothing_is_steered_twice_or_from_a_failed_entry() {
        let mut queue = queue_of(&["fix", "broken"]);
        let (fix, broken) = (queue[0].id, queue[1].id);
        queue[1].failed = true;
        assert!(begin_steer(&mut queue, fix).is_some());
        assert_eq!(begin_steer(&mut queue, fix), None);
        assert_eq!(begin_steer(&mut queue, broken), None);
    }

    /// Deleted while its steer was out: the answer finds nothing to settle.
    #[test]
    fn an_answer_for_a_deleted_entry_is_ignored() {
        let mut queue = queue_of(&["fix"]);
        let id = queue[0].id;
        begin_steer(&mut queue, id);
        queue.clear();
        assert_eq!(finish_steer(&mut queue, id, SteerResult::Delivered), None);
    }
}

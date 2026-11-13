//! Unit tests for [`super`].

use super::*;

fn queue(texts: &[&str]) -> Vec<QueuedPanelPrompt> {
    texts.iter()
         .map(|text| QueuedPanelPrompt::new((*text).to_string(), PromptOrigin::User))
         .collect()
}

fn texts(queue: &[QueuedPanelPrompt]) -> Vec<&str> {
    queue.iter().map(|prompt| prompt.text.as_str()).collect()
}

/// The user's own prompt always brings the conversation to its answer;
/// one Knot sent never pulls a reader away from where they are (#246).
#[test]
fn only_the_users_own_prompt_pulls_a_reader_to_the_tail() {
    assert!(PromptOrigin::User.follows(false));
    assert!(!PromptOrigin::InboxNudge.follows(false));
    assert!(!PromptOrigin::Startup.follows(false));
    assert!(!PromptOrigin::Autopilot.follows(false));
    // At the tail already, following costs the reader nothing.
    assert!(PromptOrigin::InboxNudge.follows(true));
    assert!(PromptOrigin::Startup.follows(true));
    assert!(PromptOrigin::Autopilot.follows(true));
}

/// A nudge that arrives mid-turn goes behind the work already waiting
/// and stays marked as a nudge: nothing downstream can tell one from a
/// user's prompt by its text, which is the whole point of the field.
#[test]
fn a_queued_nudge_waits_its_turn_and_keeps_its_origin() {
    let mut queue = queue(&["first", "second"]);
    queue.push(QueuedPanelPrompt::new("nudge".to_string(), PromptOrigin::InboxNudge));

    assert_eq!(texts(&queue), ["first", "second", "nudge"]);
    assert_eq!(queue.iter().map(|prompt| prompt.origin).collect::<Vec<_>>(),
               [PromptOrigin::User,
                PromptOrigin::User,
                PromptOrigin::InboxNudge]);
}

/// Taking the entry in front of a nudge leaves the nudge where it is -
/// the turn it was waiting behind ending is what promotes it, not
/// anything about the nudge itself.
#[test]
fn taking_the_entry_ahead_promotes_the_nudge_unchanged() {
    let mut queue = queue(&["first"]);
    queue.push(QueuedPanelPrompt::new("nudge".to_string(), PromptOrigin::InboxNudge));

    assert_eq!(take_next(&mut queue, false).map(|prompt| prompt.text)
                                           .as_deref(),
               Some("first"));

    assert_eq!(texts(&queue), ["nudge"]);
    assert_eq!(queue[0].origin, PromptOrigin::InboxNudge);
}

/// #24: once the agent has read its inbox, a nudge still waiting would
/// only show as queued for messages already picked up.
#[test]
fn dropping_nudges_keeps_the_users_prompts_in_order() {
    let mut queue = queue(&["first"]);
    queue.push(QueuedPanelPrompt::new("nudge".to_string(), PromptOrigin::InboxNudge));
    queue.push(QueuedPanelPrompt::new("second".to_string(), PromptOrigin::User));

    assert!(drop_nudges(&mut queue));
    assert_eq!(texts(&queue), ["first", "second"]);
    assert!(!drop_nudges(&mut queue), "nothing left to drop");
}

#[test]
fn removing_one_entry_keeps_the_order_of_the_rest() {
    let mut queue = queue(&["first", "second", "third"]);
    let second = queue[1].id;

    assert!(remove(&mut queue, second));

    assert_eq!(texts(&queue), ["first", "third"]);
}

#[test]
fn removing_picks_the_named_entry_among_identical_text() {
    let mut queue = queue(&["same", "same", "same"]);
    let middle = queue[1].id;
    let (first, last) = (queue[0].id, queue[2].id);

    assert!(remove(&mut queue, middle));

    assert_eq!(queue.iter().map(|prompt| prompt.id).collect::<Vec<_>>(),
               [first, last]);
}

#[test]
fn removing_an_absent_id_changes_nothing() {
    let mut queue = queue(&["first"]);

    assert!(!remove(&mut queue, Uuid::new_v4()));

    assert_eq!(texts(&queue), ["first"]);
}

#[test]
fn a_failed_entry_is_deletable() {
    let mut queue = queue(&["failed"]);
    queue[0].failed = true;
    let failed = queue[0].id;

    assert!(remove(&mut queue, failed));

    assert!(queue.is_empty());
}

#[test]
fn taking_an_entry_returns_its_text_and_keeps_the_rest_in_order() {
    let mut queue = queue(&["first", "second", "third"]);
    let second = queue[1].id;

    assert_eq!(take(&mut queue, second).as_deref(), Some("second"));

    assert_eq!(texts(&queue), ["first", "third"]);
}

#[test]
fn taking_picks_the_named_entry_among_identical_text() {
    let mut queue = queue(&["same", "same", "same"]);
    let middle = queue[1].id;
    let (first, last) = (queue[0].id, queue[2].id);

    assert_eq!(take(&mut queue, middle).as_deref(), Some("same"));

    assert_eq!(queue.iter().map(|prompt| prompt.id).collect::<Vec<_>>(),
               [first, last]);
}

#[test]
fn taking_an_absent_id_is_refused() {
    let mut queue = queue(&["first"]);

    assert_eq!(take(&mut queue, Uuid::new_v4()), None);

    assert_eq!(texts(&queue), ["first"]);
}

#[test]
fn a_failed_entry_can_be_taken_for_editing() {
    let mut queue = queue(&["failed"]);
    queue[0].failed = true;
    let failed = queue[0].id;

    assert_eq!(take(&mut queue, failed).as_deref(), Some("failed"));

    assert!(queue.is_empty());
}

/// Edited text is a new prompt: it goes behind whatever is still
/// waiting rather than reclaiming the place it was taken from.
#[test]
fn re_enqueuing_edited_text_puts_it_at_the_back() {
    let mut queue = queue(&["first", "second", "third"]);
    let first = queue[0].id;

    let text = take(&mut queue, first).expect("first should be editable");
    queue.push(QueuedPanelPrompt::new(format!("{text} (edited)"), PromptOrigin::User));

    assert_eq!(texts(&queue), ["second", "third", "first (edited)"]);
}

#[test]
fn an_empty_composer_is_replaced_without_asking() {
    assert!(!needs_replace_confirmation(""));
}

#[test]
fn a_whitespace_only_composer_is_not_work_worth_keeping() {
    assert!(!needs_replace_confirmation("  \n\t "));
}

#[test]
fn typed_composer_text_has_to_be_confirmed_before_replacing() {
    assert!(needs_replace_confirmation("half a thought"));
    assert!(needs_replace_confirmation("  padded  "));
}

/// Declining the confirmation runs no queue operation at all, so the
/// entry that was about to be edited is still queued and still first.
#[test]
fn declining_leaves_the_entry_queued() {
    let mut queue = queue(&["first", "second"]);
    let first = queue[0].id;

    if !needs_replace_confirmation("typed") {
        let _ = take(&mut queue, first);
    }

    assert_eq!(texts(&queue), ["first", "second"]);
    assert_eq!(queue[0].id, first);
}

#[test]
fn retry_clears_the_failed_mark_and_keeps_the_place() {
    let mut queue = queue(&["first", "second"]);
    queue[1].failed = true;
    let second = queue[1].id;

    assert!(retry(&mut queue, second));

    assert_eq!(texts(&queue), ["first", "second"]);
    assert!(!queue[1].failed);
    assert_eq!(queue[1].id, second);
}

#[test]
fn retry_ignores_an_entry_that_has_not_failed() {
    let mut queue = queue(&["first"]);
    let first = queue[0].id;

    assert!(!retry(&mut queue, first));
}

/// The row disappears when the prompt is sent, not when its turn ends.
#[test]
fn taking_the_head_removes_it_from_the_queue() {
    let mut queue = queue(&["first", "second"]);
    let first = queue[0].id;

    let taken = take_next(&mut queue, false).expect("the head is waiting");

    assert_eq!(taken.id, first);
    assert_eq!(texts(&queue), ["second"]);
}

#[test]
fn a_failed_head_is_not_taken() {
    let mut queue = queue(&["failed", "second"]);
    queue[0].failed = true;

    assert_eq!(take_next(&mut queue, false), None);

    assert_eq!(texts(&queue), ["failed", "second"]);
}

#[test]
fn taking_from_an_empty_queue_takes_nothing() {
    assert_eq!(take_next(&mut Vec::new(), false), None);
}

/// A failed delivery comes back as the same prompt, in front of what
/// was queued behind it while it was out.
#[test]
fn a_failed_delivery_returns_to_the_head_marked() {
    let mut queue = queue(&["first", "second"]);
    let taken = take_next(&mut queue, false).expect("the head is waiting");
    let first = taken.id;
    queue.push(QueuedPanelPrompt::new("third".to_string(), PromptOrigin::User));

    return_failed(&mut queue, taken);

    assert_eq!(texts(&queue), ["first", "second", "third"]);
    assert_eq!(queue[0].id, first);
    assert!(queue[0].failed);
    assert_eq!(take_next(&mut queue, false),
               None,
               "the failed head blocks the queue");
}

/// #368: while the session's usage limit holds it, the head waits.
#[test]
fn a_held_session_takes_nothing() {
    let mut queue = vec![QueuedPanelPrompt::new("a".into(), PromptOrigin::User)];

    assert_eq!(take_next(&mut queue, true), None);
    assert_eq!(queue.len(), 1, "the prompt is still queued");
    assert!(take_next(&mut queue, false).is_some(),
            "and goes once the limit lifts");
}

/// #368: "send now" takes the entry ahead of the rest, past the limit.
#[test]
fn send_now_moves_the_entry_to_the_head_past_the_limit() {
    let first = QueuedPanelPrompt::new("first".into(), PromptOrigin::User);
    let second = QueuedPanelPrompt::new("second".into(), PromptOrigin::User);
    let mut queue = vec![first.clone(), second.clone()];

    assert!(send_now(&mut queue, second.id));
    let taken = take_next(&mut queue, true).expect("sent now, though held");

    assert_eq!(taken.id, second.id);
    assert_eq!(take_next(&mut queue, true), None, "the rest still wait");
}

#[test]
fn send_now_leaves_a_failed_or_missing_entry_alone() {
    let mut failed = QueuedPanelPrompt::new("x".into(), PromptOrigin::User);
    failed.failed = true;
    let mut queue = vec![failed.clone()];

    assert!(!send_now(&mut queue, failed.id),
            "retry is a failed entry's way out");
    assert!(!send_now(&mut queue, Uuid::new_v4()));
}

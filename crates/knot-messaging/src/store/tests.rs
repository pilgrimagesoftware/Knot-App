//! Unit tests for [`super::MessageStore`].

use super::*;

fn msg(to: Uuid) -> Message {
    Message::new(Uuid::new_v4(), to, "hi")
}

#[test]
fn unread_filters_by_recipient_and_read_flag() {
    let mut store = MessageStore::new();
    let agent = Uuid::new_v4();
    let other = Uuid::new_v4();
    store.add(msg(agent));
    store.add(msg(other));

    let unread = store.unread_for(agent);

    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].to, agent);
}

#[test]
fn mark_read_clears_has_unread() {
    let mut store = MessageStore::new();
    let agent = Uuid::new_v4();
    store.add(msg(agent));

    assert!(store.has_unread(agent));
    store.mark_read(agent);
    assert!(!store.has_unread(agent));
}

#[test]
fn unread_count_tracks_only_unread_messages_for_recipient() {
    let mut store = MessageStore::new();
    let agent = Uuid::new_v4();
    let other = Uuid::new_v4();
    store.add(msg(agent));
    store.add(msg(agent));
    store.add(msg(other));

    assert_eq!(store.unread_count(agent), 2);
    store.mark_read(agent);
    assert_eq!(store.unread_count(agent), 0);
}

#[test]
fn latest_unread_id_is_the_most_recent() {
    let mut store = MessageStore::new();
    let agent = Uuid::new_v4();
    store.add(msg(agent));
    let second = msg(agent);
    let second_id = second.id;
    store.add(second);

    assert_eq!(store.latest_unread_id(agent), Some(second_id));
}

#[test]
fn cleanup_prunes_oldest_read_messages_keeping_unread() {
    let mut store = MessageStore::new();
    let agent = Uuid::new_v4();
    let mut unread_ids = Vec::new();
    for _ in 0..5 {
        let m = msg(agent);
        unread_ids.push(m.id);
        store.add(m);
    }
    for _ in 0..150 {
        let mut m = msg(agent);
        m.is_read = true;
        store.add(m);
    }

    store.cleanup();

    let read_count = store.messages.iter().filter(|m| m.is_read).count();
    assert_eq!(read_count, READ_RETENTION_LIMIT);
    for id in unread_ids {
        assert!(store.messages.iter().any(|m| m.id == id));
    }
}

#[test]
fn read_history_is_bounded_without_an_explicit_cleanup_call() {
    let mut store = MessageStore::new();
    let agent = Uuid::new_v4();
    for _ in 0..(READ_RETENTION_LIMIT * 3) {
        let mut message = msg(agent);
        message.is_read = true;
        store.add(message);
    }

    assert_eq!(store.messages.len(), READ_RETENTION_LIMIT);
}

#[test]
fn marking_read_prunes_the_history_it_just_created() {
    let mut store = MessageStore::new();
    let agent = Uuid::new_v4();
    for _ in 0..(READ_RETENTION_LIMIT * 2) {
        store.add(msg(agent));
    }
    assert_eq!(store.messages.len(), READ_RETENTION_LIMIT * 2);

    store.mark_read(agent);

    assert_eq!(store.messages.len(), READ_RETENTION_LIMIT);
}

/// #405: what a reset must not lose - a brief that was read and not yet
/// answered.
mod unanswered {
    use super::*;

    fn store_with_brief() -> (MessageStore, Uuid, Uuid) {
        let mut store = MessageStore::new();
        let (sender, agent) = (Uuid::new_v4(), Uuid::new_v4());
        store.add(Message::new(sender, agent, "the brief"));
        (store, sender, agent)
    }

    #[test]
    fn an_unread_brief_is_not_yet_owed_a_reply() {
        let (store, _, agent) = store_with_brief();
        assert!(!store.has_unanswered(agent, |_| true),
                "unread is the inbox's job, not this");
    }

    #[test]
    fn a_read_brief_is_unanswered_until_its_sender_hears_back() {
        let (mut store, sender, agent) = store_with_brief();
        store.mark_read(agent);
        assert_eq!(store.unanswered_for(agent, |_| true).len(), 1);

        store.add(Message::new(agent, sender, "done"));
        assert!(!store.has_unanswered(agent, |_| true));
    }

    #[test]
    fn writing_to_someone_else_does_not_answer_it() {
        let (mut store, _, agent) = store_with_brief();
        store.mark_read(agent);
        store.add(Message::new(agent, Uuid::new_v4(), "to a third agent"));
        assert!(store.has_unanswered(agent, |_| true));
    }

    #[test]
    fn a_broadcast_is_never_owed_a_reply() {
        let mut store = MessageStore::new();
        let agent = Uuid::new_v4();
        store.add(Message::broadcast(Uuid::new_v4(), agent, "fyi"));
        store.mark_read(agent);
        assert!(!store.has_unanswered(agent, |_| true));
    }

    #[test]
    fn reopening_puts_the_brief_back_in_the_inbox() {
        let (mut store, _, agent) = store_with_brief();
        store.mark_read(agent);

        let reopened = store.reopen_unanswered(agent, |_| true);

        assert_eq!(reopened.len(), 1);
        assert_eq!(reopened[0].content, "the brief");
        assert_eq!(store.unread_count(agent), 1);
        assert_eq!(store.unread_for(agent)[0].read_at, None);
        assert!(!store.has_unanswered(agent, |_| true),
                "unread again, so not owed yet");
    }

    #[test]
    fn read_messages_are_listed_only_when_asked_for() {
        let (mut store, _, agent) = store_with_brief();
        store.mark_read(agent);
        assert!(store.for_agent(agent, false, None).is_empty());
        assert_eq!(store.for_agent(agent, true, None).len(), 1);
        let later = SystemTime::now() + std::time::Duration::from_secs(60);
        assert!(store.for_agent(agent, true, Some(later)).is_empty(),
                "`since` drops what was sent before it");
    }

    /// #405 review: `dispatch-task` delivers its goal as a message, and the
    /// worker reports the task with `complete-task`, which writes nothing
    /// back. A goal that was owed a reply would hold every later reset.
    #[test]
    fn a_dispatched_tasks_goal_is_never_owed_a_reply() {
        let (mut store, sender, agent) = store_with_brief();
        let goal = Message::new(sender, agent, "the task's goal");
        let goal_id = goal.id;
        store.add(goal);
        store.mark_task(goal_id);
        store.mark_read(agent);

        let owed = store.unanswered_for(agent, |_| true);
        assert_eq!(owed.len(), 1, "only the plain brief is owed");
        assert_eq!(owed[0].content, "the brief");
    }

    /// A sender that is gone can never be answered.
    #[test]
    fn a_brief_from_an_agent_that_is_gone_is_not_owed() {
        let (mut store, sender, agent) = store_with_brief();
        store.mark_read(agent);

        assert!(!store.has_unanswered(agent, |id| id != sender));
        assert!(store.reopen_unanswered(agent, |id| id != sender).is_empty());
    }
}

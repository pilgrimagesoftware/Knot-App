use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An agent-to-agent message. Held in memory only; does not persist across
/// an app restart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id:        Uuid,
    pub from:      Uuid,
    pub to:        Uuid,
    pub content:   String,
    pub timestamp: SystemTime,
    pub is_read:   bool,
    /// When the recipient read it, or `None` while unread. What a reply
    /// is measured against: a message answered is one the recipient wrote
    /// back to its sender after reading it (#405).
    pub read_at:   Option<SystemTime>,
    /// One of a broadcast's copies. A broadcast tells, it does not ask, so
    /// it is never left unanswered.
    pub broadcast: bool,
    /// A dispatched task's goal, delivered by `dispatch-task`. Never owed a
    /// reply either: the task graph tracks that work, and the worker reports
    /// it with `complete-task`, which writes no message back.
    pub task:      bool,
}

impl Message {
    /// Mints a fresh unread message.
    pub fn new(from: Uuid, to: Uuid, content: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(),
               from,
               to,
               content: content.into(),
               timestamp: SystemTime::now(),
               is_read: false,
               read_at: None,
               broadcast: false,
               task: false }
    }

    /// One recipient's copy of a broadcast: unread, and never owed a reply.
    pub fn broadcast(from: Uuid, to: Uuid, content: impl Into<String>) -> Self {
        Self { broadcast: true,
               ..Self::new(from, to, content) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_message_is_unread() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();

        let message = Message::new(from, to, "hello");

        assert!(!message.is_read);
        assert_eq!(message.from, from);
        assert_eq!(message.to, to);
        assert_eq!(message.content, "hello");
    }

    #[test]
    fn each_message_gets_a_fresh_id() {
        let a = Message::new(Uuid::new_v4(), Uuid::new_v4(), "a");
        let b = Message::new(Uuid::new_v4(), Uuid::new_v4(), "b");

        assert_ne!(a.id, b.id);
    }
}

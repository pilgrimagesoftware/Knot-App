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
    /// What kind of message it is, which decides whether it is owed a reply.
    pub kind:      MessageKind,
}

/// What a message is, for whether its recipient owes the sender a reply
/// (#405). One per message: a message is exactly one of these.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    /// Sent to this agent alone with `send-message`: owed a reply.
    #[default]
    Direct,
    /// One of a broadcast's copies. A broadcast tells, it does not ask.
    Broadcast,
    /// A dispatched task's goal, delivered by `dispatch-task`. The task graph
    /// tracks that work, and the worker reports it with `complete-task`,
    /// which writes no message back.
    Task,
    /// Written by Knot on an agent's behalf, such as telling a sender that
    /// the agent's conversation was reset. Neither owed a reply nor a reply
    /// itself: the agent did not write it.
    Notice,
}

impl MessageKind {
    /// Whether a message of this kind, once read, is owed a reply.
    pub fn owes_reply(self) -> bool {
        match self {
            Self::Direct => true,
            Self::Broadcast | Self::Task | Self::Notice => false,
        }
    }

    /// Whether a message of this kind, sent back to someone, answers them.
    pub fn answers(self) -> bool {
        match self {
            Self::Direct | Self::Broadcast | Self::Task => true,
            Self::Notice => false,
        }
    }
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
               kind: MessageKind::Direct }
    }

    /// One recipient's copy of a broadcast: unread, and never owed a reply.
    pub fn broadcast(from: Uuid, to: Uuid, content: impl Into<String>) -> Self {
        Self { kind: MessageKind::Broadcast,
               ..Self::new(from, to, content) }
    }

    /// A notice Knot writes on `from`'s behalf; see [`MessageKind::Notice`].
    pub fn notice(from: Uuid, to: Uuid, content: impl Into<String>) -> Self {
        Self { kind: MessageKind::Notice,
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

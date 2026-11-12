use std::time::SystemTime;

use uuid::Uuid;

use crate::consts::READ_RETENTION_LIMIT;
use crate::message::Message;

/// In-memory message queue. No persistence: a fresh `MessageStore` is the
/// entire history on every app restart.
#[derive(Debug, Clone, Default)]
pub struct MessageStore {
    messages: Vec<Message>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a message, pruning read history past
    /// [`READ_RETENTION_LIMIT`] on the way.
    ///
    /// The prune happens here rather than being left to a caller: nothing
    /// in the app called [`MessageStore::cleanup`], so `messages` grew
    /// without bound for the whole life of the process. Every operation
    /// that can enlarge the prunable set - this and
    /// [`MessageStore::mark_read`] - sweeps, so the retention bound holds
    /// without a caller having to remember to ask for it.
    pub fn add(&mut self, message: Message) {
        self.messages.push(message);
        self.cleanup();
    }

    pub fn unread_for(&self, agent_id: Uuid) -> Vec<&Message> {
        self.messages
            .iter()
            .filter(|m| m.to == agent_id && !m.is_read)
            .collect()
    }

    pub fn unread_count(&self, agent_id: Uuid) -> usize {
        self.messages
            .iter()
            .filter(|message| message.to == agent_id && !message.is_read)
            .count()
    }

    /// Marks every message for `agent_id` read, stamping when, then prunes -
    /// marking is the other way the prunable set grows. See
    /// [`MessageStore::add`].
    pub fn mark_read(&mut self, agent_id: Uuid) {
        let now = SystemTime::now();
        for m in self.messages
                     .iter_mut()
                     .filter(|m| m.to == agent_id && !m.is_read)
        {
            m.is_read = true;
            m.read_at = Some(now);
        }
        self.cleanup();
    }

    /// Marks the messages `ids` names read, stamping when, then prunes. For
    /// a check that returned only some of an agent's unread messages: one
    /// it filtered out was never shown, so it stays unread (#405).
    pub fn mark_read_ids(&mut self, ids: &[Uuid]) {
        let now = SystemTime::now();
        for m in self.messages
                     .iter_mut()
                     .filter(|m| !m.is_read && ids.contains(&m.id))
        {
            m.is_read = true;
            m.read_at = Some(now);
        }
        self.cleanup();
    }

    /// `agent_id`'s messages, oldest first: the unread ones, and the read
    /// ones too when `include_read`, each sent at or after `since` when it
    /// is given. What `check-messages` returns, so a conversation that
    /// started after a brief was read can still find it (#405).
    pub fn for_agent(&self, agent_id: Uuid, include_read: bool, since: Option<SystemTime>)
                     -> Vec<&Message> {
        self.messages
            .iter()
            .filter(|m| m.to == agent_id && (include_read || !m.is_read))
            .filter(|m| since.is_none_or(|since| m.timestamp >= since))
            .collect()
    }

    /// Marks message `id` as a dispatched task's goal, which is never owed
    /// a reply (see [`Message::task`]).
    pub fn mark_task(&mut self, id: Uuid) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.id == id) {
            message.task = true;
        }
    }

    /// The messages `agent_id` has read and not answered, oldest first: sent
    /// to it directly by another agent that `sender_present` says still
    /// exists, not a broadcast or a dispatched task's goal, with no message
    /// from it back to that sender since it read them (#405).
    ///
    /// A reply is any message to the sender, not one that quotes the brief:
    /// messages carry no thread, and an agent reports back to whoever
    /// briefed it. So one reply settles every earlier message from that
    /// sender. A sender that is gone can never be answered, so its briefs
    /// are not owed - holding a reset for them would hold it forever.
    pub fn unanswered_for(&self, agent_id: Uuid, sender_present: impl Fn(Uuid) -> bool)
                          -> Vec<&Message> {
        self.messages
            .iter()
            .filter(|m| {
                m.to == agent_id && m.is_read && !m.broadcast && !m.task && m.from != agent_id
            })
            .filter(|m| sender_present(m.from))
            .filter(|m| !self.answered(m))
            .collect()
    }

    /// Whether `agent_id` holds a message it has read and not answered; see
    /// [`Self::unanswered_for`].
    pub fn has_unanswered(&self, agent_id: Uuid, sender_present: impl Fn(Uuid) -> bool) -> bool {
        !self.unanswered_for(agent_id, sender_present).is_empty()
    }

    /// Marks `agent_id`'s unanswered messages unread again and hands them
    /// back, for a conversation that is about to be replaced: the new one
    /// finds them in its inbox, whole, rather than not at all (#405).
    pub fn reopen_unanswered(&mut self, agent_id: Uuid, sender_present: impl Fn(Uuid) -> bool)
                             -> Vec<Message> {
        let ids: Vec<Uuid> = self.unanswered_for(agent_id, sender_present)
                                 .iter()
                                 .map(|m| m.id)
                                 .collect();
        let mut reopened = Vec::with_capacity(ids.len());
        for m in self.messages.iter_mut().filter(|m| ids.contains(&m.id)) {
            m.is_read = false;
            m.read_at = None;
            reopened.push(m.clone());
        }
        reopened
    }

    /// Whether `message`'s recipient has written to its sender since reading
    /// it.
    fn answered(&self, message: &Message) -> bool {
        let since = message.read_at.unwrap_or(message.timestamp);
        self.messages.iter().any(|reply| {
                                reply.from == message.to
                                && reply.to == message.from
                                && reply.timestamp >= since
                            })
    }

    pub fn has_unread(&self, agent_id: Uuid) -> bool {
        self.messages.iter().any(|m| m.to == agent_id && !m.is_read)
    }

    pub fn latest_unread_id(&self, agent_id: Uuid) -> Option<Uuid> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.to == agent_id && !m.is_read)
            .map(|m| m.id)
    }

    /// Prunes the oldest read messages beyond [`READ_RETENTION_LIMIT`].
    /// Unread messages are never touched.
    pub fn cleanup(&mut self) {
        let read_count = self.messages.iter().filter(|m| m.is_read).count();
        if read_count <= READ_RETENTION_LIMIT {
            return;
        }
        let mut to_remove = read_count - READ_RETENTION_LIMIT;
        self.messages.retain(|m| {
                         if m.is_read && to_remove > 0 {
                             to_remove -= 1;
                             false
                         }
                         else {
                             true
                         }
                     });
    }
}

#[cfg(test)]
mod tests;

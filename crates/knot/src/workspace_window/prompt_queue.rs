//! An agent panel's queue of prompts waiting for their turn.
//!
//! Every entry carries a stable identity, so each mutation names the entry
//! it means instead of a position. Deletion, a delivery result arriving off
//! the runtime, and the pump promoting the head all reach the queue
//! independently, and any one of them shifts every index behind it. Text is
//! no better as an identity: two prompts that read the same are ordinary,
//! and matching on text picks whichever comes first.
//!
//! Contract: `openspec/specs/queued-message-management/spec.md`.

use uuid::Uuid;

/// What put a prompt in the queue.
///
/// Carried explicitly rather than recovered from the prompt text, per
/// `mcp-messaging`'s "Inbox nudges preserve interrupted session work": an
/// automatic nudge and a user prompt that reads the same are
/// indistinguishable after the fact, so matching on the nudge's wording
/// would reclassify a user who pasted it.
///
/// Never serialized - the queue lives only as long as the window - so
/// unlike the vocabularies in `.claude/rules/rust-structure.md` this one
/// needs no `Display`/`FromStr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PromptOrigin {
    /// Typed by the user, or sent on the user's behalf by a broadcast.
    #[default]
    User,
    /// The automatic "check your inbox" nudge Knot sends of its own accord.
    InboxNudge,
    /// The agent's startup prompt, queued behind its registration turn on a
    /// fresh session. See `openspec/specs/agent-launch-command/spec.md`.
    Startup,
    /// A reply Autopilot wrote for the user after classifying the agent's
    /// last turn. See `openspec/specs/autopilot/spec.md`.
    Autopilot,
}

impl PromptOrigin {
    /// Whether delivering a prompt of this origin brings the conversation to
    /// its newest message and follows the answer (#246).
    ///
    /// A prompt the user sent does: they asked for that answer. A prompt
    /// Knot sends of its own accord - an inbox nudge, a startup or
    /// registration prompt - follows only a reader who is already at the
    /// tail. Someone reading back through the history keeps their place,
    /// exactly as they would for streamed output.
    pub(crate) fn follows(self, reader_at_tail: bool) -> bool {
        match self {
            PromptOrigin::User => true,
            PromptOrigin::InboxNudge | PromptOrigin::Startup | PromptOrigin::Autopilot => {
                reader_at_tail
            }
        }
    }
}

/// One prompt waiting to be delivered to an agent.
///
/// An entry leaves the queue the moment the pump hands it to the agent -
/// the prompt is in the conversation from then on, and a row left behind
/// for the length of the turn read as one still waiting. `failed` marks
/// one whose delivery returned an error; [`return_failed`] puts it back at
/// the head, where it holds its position until the user retries or deletes
/// it, rather than being retried automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueuedPanelPrompt {
    pub(crate) id:       Uuid,
    pub(crate) text:     String,
    pub(crate) failed:   bool,
    /// What put this prompt here. Set at the one point where that is still
    /// known; nothing downstream can recover it.
    pub(crate) origin:   PromptOrigin,
    /// The user chose to send this now, though the session's usage limit
    /// still stands (#368). The pump hands it over without waiting for the
    /// limit to lift.
    pub(crate) send_now: bool,
    /// Handed to the running turn with a steer whose answer has not come
    /// back (#26). Not taken at turn end - that would send it twice - and
    /// not steered again.
    pub(crate) steering: bool,
}

impl QueuedPanelPrompt {
    pub(crate) fn new(text: String, origin: PromptOrigin) -> Self {
        Self { id: Uuid::new_v4(),
               text,
               failed: false,
               origin,
               send_now: false,
               steering: false }
    }
}

/// Removes the entry `id` names, leaving the rest of the queue in order.
///
/// Returns whether an entry was removed. An `id` that is already gone - a
/// delivery that landed between the click and this call - is not an error.
pub(crate) fn remove(queue: &mut Vec<QueuedPanelPrompt>, id: Uuid) -> bool {
    let Some(index) = position(queue, id)
    else {
        return false;
    };
    queue.remove(index);
    true
}

/// Removes the entry `id` names and returns its text, for editing.
///
/// `None` means the entry is gone - already handed to the agent, which
/// cannot be taken back - and the caller leaves the composer as it found
/// it. The entry does not hold its place - edited text is sent as a new
/// prompt, at the back.
pub(crate) fn take(queue: &mut Vec<QueuedPanelPrompt>, id: Uuid) -> Option<String> {
    let index = position(queue, id)?;
    Some(queue.remove(index).text)
}

/// Clears the failed mark on `id` so the pump offers the entry again.
///
/// The prompt keeps its place and its identity; retry re-delivers the same
/// message rather than enqueuing a new one.
pub(crate) fn retry(queue: &mut [QueuedPanelPrompt], id: Uuid) -> bool {
    let Some(prompt) = queue.iter_mut().find(|prompt| prompt.id == id)
    else {
        return false;
    };
    if !prompt.failed {
        return false;
    }
    prompt.failed = false;
    true
}

/// Takes the head of the queue for delivery.
///
/// A failed head is not taken: it waits for the user to retry or delete it,
/// and everything behind it waits too, so prompts are never delivered out
/// of the order they were queued in. Nor is a head while the session is
/// `held` by its usage limit, unless the user chose to send it now (#368).
///
/// An entry being steered into the running turn is passed over (#26): it is
/// already on its way, and its answer either removes it or puts it back at
/// the head.
pub(crate) fn take_next(queue: &mut Vec<QueuedPanelPrompt>, held: bool)
                        -> Option<QueuedPanelPrompt> {
    let index = queue.iter().position(|prompt| !prompt.steering)?;
    let head = &queue[index];
    if head.failed || (held && !head.send_now) {
        return None;
    }
    Some(queue.remove(index))
}

/// The user's override for a session held by its usage limit (#368): `id`
/// goes to the head of the queue, marked to be sent now. Returns whether
/// the entry was found.
///
/// To the head, because "now" means next - it would otherwise still wait
/// behind entries the limit is holding. A failed entry is left alone: retry
/// is its way out.
pub(crate) fn send_now(queue: &mut Vec<QueuedPanelPrompt>, id: Uuid) -> bool {
    let Some(index) = position(queue, id)
    else {
        return false;
    };
    if queue[index].failed {
        return false;
    }
    let mut prompt = queue.remove(index);
    prompt.send_now = true;
    queue.insert(0, prompt);
    true
}

/// Puts a prompt whose delivery failed back at the head of the queue,
/// marked failed, with its text and identity intact.
///
/// The head, because it was the head when it was taken: the pump takes
/// nothing else while a delivery is outstanding, so everything still queued
/// was behind it.
pub(crate) fn return_failed(queue: &mut Vec<QueuedPanelPrompt>, mut prompt: QueuedPanelPrompt) {
    prompt.failed = true;
    queue.insert(0, prompt);
}

/// Drops every inbox nudge still waiting in `queue`, reporting whether any
/// went (#24).
///
/// For an agent whose inbox has been read since the nudge was queued: the
/// nudge asks it to read messages it has already picked up, and the row it
/// leaves above the composer says the opposite of what has happened. The
/// rest of the queue keeps its order. A later unread message is nudged
/// afresh by the poll, so nothing is lost.
pub(crate) fn drop_nudges(queue: &mut Vec<QueuedPanelPrompt>) -> bool {
    let before = queue.len();
    queue.retain(|prompt| prompt.origin != PromptOrigin::InboxNudge);
    queue.len() != before
}

fn position(queue: &[QueuedPanelPrompt], id: Uuid) -> Option<usize> {
    queue.iter().position(|prompt| prompt.id == id)
}

/// Whether loading a queued message into the composer would destroy work,
/// and so has to be confirmed first.
///
/// Whitespace alone is not work: a stray newline in an untouched composer
/// should not make every edit ask.
pub(crate) fn needs_replace_confirmation(composer: &str) -> bool {
    !composer.trim().is_empty()
}

/// The localized name of a queued entry's state, for the row's status icon.
///
/// The icon carries the state visually and the row's only text is the
/// prompt itself, so this is the sole thing that tells a screen reader
/// whether the entry is waiting or stuck.
///
/// A queued inbox nudge says so. Since nudges queue behind a running turn
/// the user sees a prompt in their own queue that they never typed, and the
/// row's text - the nudge's wording - is the only other clue.
///
/// `failed` wins over the origin: a stuck entry is the state the user has
/// to act on, and where it came from does not change that.
pub(crate) fn queued_status_label(failed: bool, origin: PromptOrigin) -> String {
    knot_core::l10n::t(match (failed, origin) {
                           (true, _) => "panel.failed",
                           (false, PromptOrigin::InboxNudge) => "panel.queued_inbox_nudge",
                           (false, PromptOrigin::Startup) => "panel.queued_startup_prompt",
                           (false, PromptOrigin::Autopilot) => "panel.queued_autopilot",
                           (false, PromptOrigin::User) => "panel.queued",
                       })
}

mod steering;
pub(crate) use steering::{SteerResult, begin_steer, finish_steer};

#[cfg(test)]
mod tests;

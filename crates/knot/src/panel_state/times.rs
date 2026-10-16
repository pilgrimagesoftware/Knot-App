//! When each conversation entry happened, kept across a restored session
//! (#220).
//!
//! An agent replays a loaded session's history as ordinary session updates,
//! with no times attached, so every replayed entry used to be stamped with the
//! moment it was replayed and read "just now". Knot records the time of each
//! entry as it happens live, the session layer persists that log per ACP
//! session, and a restored session hands it back here to reconcile against
//! what the agent replays.
//!
//! Reconciliation anchors on identity rather than position. A replay is not
//! the live conversation entry for entry: local-only entries (shell commands,
//! notices, errors) are never replayed, and a prompt the agent's harness
//! injected may replay as a notice or not at all. So a prompt is matched by a
//! hash of its text and a tool call by its id; an assistant message takes the
//! logged reply that directly follows its matched anchor. Anything that does
//! not match gets no time - shown without one, rather than with a wrong one.
//!
//! Pure: no I/O. `panel_session::times_store` reads and writes the log.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::message::PanelMessage;

/// One logged entry: what it was, enough to recognise it on replay, and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TimedEntry {
    /// A prompt the user sent, recognised by [`prompt_hash`] of its text.
    #[serde(rename_all = "camelCase")]
    Prompt { hash: u64, at_ms: u64 },
    /// The start of a stretch of assistant text.
    #[serde(rename_all = "camelCase")]
    Reply { at_ms: u64 },
    /// A tool call, recognised by its ACP tool-call id.
    #[serde(rename_all = "camelCase")]
    ToolCall { id: String, at_ms: u64 },
}

impl TimedEntry {
    fn at(&self) -> SystemTime {
        let ms = match self {
            Self::Prompt { at_ms, .. } | Self::Reply { at_ms } | Self::ToolCall { at_ms, .. } => {
                *at_ms
            }
        };
        UNIX_EPOCH + Duration::from_millis(ms)
    }

    /// The entry `message` would log at `at`, for the kinds that replay.
    fn for_message(message: &PanelMessage, at: SystemTime) -> Option<Self> {
        let at_ms =
            at.duration_since(UNIX_EPOCH).map_or(0, |since| {
                                             u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
                                         });
        match message {
            PanelMessage::User(text) => Some(Self::Prompt { hash: prompt_hash(text),
                                                            at_ms }),
            PanelMessage::Assistant(_) => Some(Self::Reply { at_ms }),
            PanelMessage::ToolCall(card) => Some(Self::ToolCall { id: card.id.clone(),
                                                                  at_ms }),
            PanelMessage::Shell(_) | PanelMessage::Error(_) | PanelMessage::Notice(_) => None,
        }
    }
}

/// A stable hash of a prompt's text: FNV-1a over its trimmed bytes.
///
/// Stable across builds, unlike `std`'s hasher, because it is stored. Trimmed
/// because a replay that splits a prompt into chunks may not keep the edges'
/// whitespace exactly.
pub fn prompt_hash(text: &str) -> u64 {
    const OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01B3;
    text.trim().bytes().fold(OFFSET, |hash, byte| {
                           (hash ^ u64::from(byte)).wrapping_mul(PRIME)
                       })
}

/// The time bookkeeping a [`super::PanelState`] carries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntryTimes {
    /// Every logged entry of the session, oldest first: what is persisted.
    log:       Vec<TimedEntry>,
    /// A restored session's log, waiting for the replay to finish.
    pending:   Option<Vec<TimedEntry>>,
    /// Whether entries are happening now. False while a restored session's
    /// history is being replayed.
    replaying: bool,
}

impl EntryTimes {
    /// A restored session's bookkeeping: what is pushed is a replay until the
    /// first live prompt, which reconciles it against `log`.
    pub fn restoring(log: Vec<TimedEntry>) -> Self {
        Self { log:       Vec::new(),
               pending:   Some(log),
               replaying: true, }
    }

    /// The session turned out not to be restored after all (the prior one
    /// could not be loaded): nothing replays, and the old log is not this
    /// session's.
    pub fn discard_restore(&mut self) {
        self.pending = None;
        self.replaying = false;
    }

    /// The time to give `message` as it is pushed - `None` while replaying,
    /// until [`Self::go_live`] reconciles it - logging it when it is live.
    pub(super) fn stamp(&mut self, message: &PanelMessage) -> Option<SystemTime> {
        if self.replaying {
            return None;
        }
        let now = SystemTime::now();
        self.log.extend(TimedEntry::for_message(message, now));
        Some(now)
    }

    /// The replay is over: give each replayed entry its logged time, and log
    /// from here on. Idempotent, so every live entry point may call it.
    pub(super) fn go_live(&mut self, messages: &[PanelMessage],
                          sent_at: &mut [Option<SystemTime>]) {
        if !self.replaying {
            return;
        }
        self.replaying = false;
        let Some(restored) = self.pending.take()
        else {
            return;
        };
        reconcile(&restored, messages, sent_at);
        // The restored log stays the session's history, matched or not, so a
        // later restore still has the entries this one could not place.
        let live = std::mem::take(&mut self.log);
        self.log = restored;
        self.log.extend(live);
    }

    /// The entries logged live so far - empty while a restored session is
    /// still replaying, whose own entries are in [`Self::snapshot`].
    #[cfg(test)]
    pub fn log(&self) -> &[TimedEntry] {
        &self.log
    }

    /// Everything to persist for this session: a restored log not yet
    /// reconciled, then what has been logged live.
    ///
    /// Not [`Self::log`] alone. A restored session that ends - or sees a turn
    /// end - before its first live prompt has logged nothing live, and saving
    /// that would overwrite the restored times with an empty document: open a
    /// conversation, look at it, close it, and every real time is gone.
    pub fn snapshot(&self) -> Vec<TimedEntry> {
        self.pending
            .iter()
            .flatten()
            .chain(&self.log)
            .cloned()
            .collect()
    }
}

/// Gives the replayed `messages` their times from `log`, writing `sent_at`.
fn reconcile(log: &[TimedEntry], messages: &[PanelMessage], sent_at: &mut [Option<SystemTime>]) {
    let mut cursor = 0;
    // Whether the last prompt or tool call was found in the log, so the reply
    // after it may take the logged reply after its match.
    let mut anchored = false;
    for (message, slot) in messages.iter().zip(sent_at.iter_mut()) {
        match message {
            PanelMessage::User(text) => {
                let hash = prompt_hash(text);
                let found = find_from(log,
                                      cursor,
                                      |entry| matches!(entry, TimedEntry::Prompt { hash: logged, .. } if *logged == hash));
                anchored = place(log, found, &mut cursor, slot);
            }
            PanelMessage::ToolCall(card) => {
                let found = find_from(log,
                                      cursor,
                                      |entry| matches!(entry, TimedEntry::ToolCall { id, .. } if *id == card.id));
                anchored = place(log, found, &mut cursor, slot);
            }
            PanelMessage::Assistant(_) => {
                if anchored && matches!(log.get(cursor), Some(TimedEntry::Reply { .. })) {
                    *slot = Some(log[cursor].at());
                    cursor += 1;
                }
                else {
                    anchored = false;
                }
            }
            PanelMessage::Shell(_) | PanelMessage::Error(_) | PanelMessage::Notice(_) => {}
        }
    }
}

/// The index of the first entry at or after `from` that `matches`.
fn find_from(log: &[TimedEntry], from: usize, matches: impl Fn(&TimedEntry) -> bool)
             -> Option<usize> {
    log.iter()
       .skip(from)
       .position(matches)
       .map(|offset| from + offset)
}

/// Gives `slot` the time of the matched entry and moves `cursor` past it,
/// reporting whether there was one.
fn place(log: &[TimedEntry], found: Option<usize>, cursor: &mut usize,
         slot: &mut Option<SystemTime>)
         -> bool {
    let Some(index) = found
    else {
        return false;
    };
    *slot = Some(log[index].at());
    *cursor = index + 1;
    true
}

#[cfg(test)]
mod tests;

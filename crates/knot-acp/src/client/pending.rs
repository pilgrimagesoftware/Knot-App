//! Requests the agent is waiting on the user for - permission requests and
//! form elicitations - by JSON-RPC id, each with the channel its answer goes
//! back through, plus the sessions this client has open.
//!
//! Every way out of a pending request goes through here: the user's answer,
//! Stop ([`PendingInputs::cancel_all`]), the session ending
//! ([`PendingInputs::end_all`]) and the agent withdrawing it
//! ([`PendingInputs::withdraw`]). Whichever comes first removes the entry, so
//! a request is answered exactly once.

use std::collections::{HashMap, HashSet};

use parking_lot::Mutex;
use tokio::sync::oneshot;

use crate::protocol::{FormAnswer, PermissionDecision};

/// How a pending `session/request_permission` gets answered.
pub(super) enum PermissionAnswer {
    Decided(PermissionDecision),
    /// The turn was cancelled: the `cancelled` outcome the spec requires.
    Cancelled,
    /// The agent sent `$/cancel_request` for it.
    Withdrawn,
}

/// How a pending `elicitation/create` gets answered.
pub(super) enum QuestionAnswer {
    Answered(FormAnswer),
    /// The agent sent `$/cancel_request` for it.
    Withdrawn,
}

enum PendingInput {
    Permission(oneshot::Sender<PermissionAnswer>),
    Question(oneshot::Sender<QuestionAnswer>),
}

#[derive(Default)]
pub(super) struct PendingInputs {
    by_id:    Mutex<HashMap<String, PendingInput>>,
    sessions: Mutex<HashSet<String>>,
}

impl PendingInputs {
    pub(super) fn add_permission(&self, key: String) -> oneshot::Receiver<PermissionAnswer> {
        let (sender, receiver) = oneshot::channel();
        self.by_id
            .lock()
            .insert(key, PendingInput::Permission(sender));
        receiver
    }

    pub(super) fn add_question(&self, key: String) -> oneshot::Receiver<QuestionAnswer> {
        let (sender, receiver) = oneshot::channel();
        self.by_id
            .lock()
            .insert(key, PendingInput::Question(sender));
        receiver
    }

    /// Delivers the user's decision, if a permission request with this id is
    /// still pending.
    pub(super) fn answer_permission(&self, key: &str, decision: PermissionDecision) {
        if let Some(PendingInput::Permission(sender)) =
            self.take(key, |entry| matches!(entry, PendingInput::Permission(_)))
        {
            let _ = sender.send(PermissionAnswer::Decided(decision));
        }
    }

    /// Delivers the user's answer, if a form with this id is still pending.
    pub(super) fn answer_question(&self, key: &str, answer: FormAnswer) {
        if let Some(PendingInput::Question(sender)) =
            self.take(key, |entry| matches!(entry, PendingInput::Question(_)))
        {
            let _ = sender.send(QuestionAnswer::Answered(answer));
        }
    }

    /// Removes the entry for `key` when `wanted` says it is the right kind,
    /// deciding under one lock so nothing can withdraw it in between.
    fn take(&self, key: &str, wanted: impl Fn(&PendingInput) -> bool) -> Option<PendingInput> {
        let mut by_id = self.by_id.lock();
        by_id.get(key)
             .is_some_and(wanted)
             .then(|| by_id.remove(key))
             .flatten()
    }

    /// Stop: permissions get `cancelled`, forms `cancel`.
    pub(super) fn cancel_all(&self) {
        self.drain(|| PermissionAnswer::Cancelled);
    }

    /// The session ended: permissions are denied (as before), forms
    /// cancelled.
    pub(super) fn end_all(&self) {
        self.drain(|| PermissionAnswer::Decided(PermissionDecision::Deny));
    }

    fn drain(&self, permission: impl Fn() -> PermissionAnswer) {
        let pending: Vec<_> = self.by_id.lock().drain().collect();
        for (_, entry) in pending {
            match entry {
                PendingInput::Permission(sender) => {
                    let _ = sender.send(permission());
                }
                PendingInput::Question(sender) => {
                    let _ = sender.send(QuestionAnswer::Answered(FormAnswer::Cancelled));
                }
            }
        }
    }

    /// `$/cancel_request` for `key`: answers it as withdrawn. `false` when
    /// nothing with that id is pending.
    pub(super) fn withdraw(&self, key: &str) -> bool {
        let Some(entry) = self.by_id.lock().remove(key)
        else {
            return false;
        };
        match entry {
            PendingInput::Permission(sender) => {
                let _ = sender.send(PermissionAnswer::Withdrawn);
            }
            PendingInput::Question(sender) => {
                let _ = sender.send(QuestionAnswer::Withdrawn);
            }
        }
        true
    }

    pub(super) fn open_session(&self, session_id: &str) {
        self.sessions.lock().insert(session_id.to_owned());
    }

    pub(super) fn has_session(&self, session_id: &str) -> bool {
        self.sessions.lock().contains(session_id)
    }
}

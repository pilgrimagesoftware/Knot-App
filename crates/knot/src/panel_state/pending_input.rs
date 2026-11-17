//! What the agent is waiting on the user for: a permission request or a
//! question card, one on screen at a time and the rest queued in arrival
//! order (`acp-panel-ui`'s "A pending question gates the session like a
//! pending permission").
//!
//! Every gate - the composer, queued delivery, steering, Awaiting input,
//! scheduled runs, idle shutdown - asks [`PanelState::has_pending_input`],
//! so a question blocks exactly what a permission does without a check of
//! its own at each site. Only the permission keys ask
//! [`PanelState::pending_permission`].

use knot_acp::{PermissionRequest, QuestionRequest};
use serde_json::Value;

use super::question::QuestionState;
use super::state::PanelState;

#[derive(Debug, Clone, PartialEq)]
pub enum PendingInput {
    Permission(PermissionRequest),
    Question(QuestionState),
}

impl PendingInput {
    pub fn rpc_id(&self) -> &Value {
        match self {
            Self::Permission(request) => &request.rpc_id,
            Self::Question(question) => &question.request.rpc_id,
        }
    }
}

impl From<PermissionRequest> for PendingInput {
    fn from(request: PermissionRequest) -> Self {
        Self::Permission(request)
    }
}

impl From<QuestionRequest> for PendingInput {
    fn from(request: QuestionRequest) -> Self {
        Self::Question(QuestionState::new(request))
    }
}

impl PanelState {
    /// Whether the agent is waiting on the user for anything: sending,
    /// queued delivery and steering all hold while it is.
    pub fn has_pending_input(&self) -> bool {
        self.pending_input.is_some()
    }

    /// The permission request on screen, if that is what is showing.
    pub fn pending_permission(&self) -> Option<&PermissionRequest> {
        match self.pending_input.as_ref()? {
            PendingInput::Permission(request) => Some(request),
            PendingInput::Question(_) => None,
        }
    }

    /// The question card on screen, if that is what is showing.
    pub fn pending_question(&self) -> Option<&QuestionState> {
        match self.pending_input.as_ref()? {
            PendingInput::Question(question) => Some(question),
            PendingInput::Permission(_) => None,
        }
    }

    pub fn pending_question_mut(&mut self) -> Option<&mut QuestionState> {
        match self.pending_input.as_mut()? {
            PendingInput::Question(question) => Some(question),
            PendingInput::Permission(_) => None,
        }
    }

    /// Clears what is on screen once the caller has answered it, per the
    /// "User denies a permission request" scenario: the prompt is replaced
    /// with its resolved state, not left pending. The next queued request,
    /// if any, takes its place.
    pub fn resolve_input(&mut self) {
        self.pending_input = self.queued_inputs.pop_front();
    }

    /// Records a request the agent is waiting on: shown at once when nothing
    /// else is, queued behind the one on screen otherwise. An agent running
    /// tool calls in parallel asks for several at once, and each waits on
    /// its own answer (#267).
    pub(super) fn request_input(&mut self, input: PendingInput) {
        if self.pending_input.is_some() {
            self.queued_inputs.push_back(input);
        }
        else {
            self.pending_input = Some(input);
        }
    }

    /// Forgets the request with `rpc_id` once the client reports it answered,
    /// by any path, including the ones that never touch this state: Stop,
    /// the session ending, the agent withdrawing it. Without it a request
    /// answered that way stayed pending here, and the agent showed "Awaiting
    /// input" until its turn ended, or for good (#344).
    ///
    /// A request the UI already resolved is gone by now, so this is then a
    /// no-op: it matches by id, never "whichever is showing".
    pub(super) fn forget_input(&mut self, rpc_id: &Value) {
        if self.pending_input
               .as_ref()
               .is_some_and(|input| input.rpc_id() == rpc_id)
        {
            self.resolve_input();
        }
        else {
            self.queued_inputs.retain(|input| input.rpc_id() != rpc_id);
        }
    }

    /// Forgets every pending request: a finished turn has none left
    /// outstanding, since the agent only ends one after each is answered.
    pub(super) fn clear_inputs(&mut self) {
        self.pending_input = None;
        self.queued_inputs.clear();
    }
}

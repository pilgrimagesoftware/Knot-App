//! Steering: handing a prompt to the turn that is running, rather than
//! queueing it as the next `session/prompt` (#26).
//!
//! Not in the stable ACP spec. It is the `_session/steering` extension that
//! `claude-agent-acp`, `codex-acp` and opencode's ACP agent implement, and
//! advertise as `InitializeResponse._meta.steering.supported`. ACP v2's prompt
//! RFD (`docs/rfds/v2/prompt.mdx`) standardises the same idea. Shapes here
//! follow `claude-agent-acp` 0.101's `parseSteerRequest` and `steer()`.

use serde_json::{Value, json};

/// The extension request's method.
pub const STEER_METHOD: &str = "_session/steering";

/// Whether an `initialize` response's top-level `_meta` advertises steering:
/// `{"steering": {"supported": true}}`. Anything else - no `_meta`, no
/// `steering`, `supported` absent or not `true` - is no.
pub fn advertises_steering(meta: Option<&Value>) -> bool {
    meta.and_then(|meta| meta.get("steering"))
        .and_then(|steering| steering.get("supported"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The `_session/steering` params for `text`, asking for `promptRequired`
/// when no turn is running: the prompt then comes back to the client, which
/// sends it as a turn of its own. Without that opt-in the adapter starts a
/// detached turn the client never sees end.
pub fn steer_params(session_id: &str, text: &str) -> Value {
    json!({
        "sessionId": session_id,
        "prompt": [{ "type": "text", "text": text }],
        "_meta": { "steering": { "idleBehavior": "promptRequired" } }
    })
}

/// What a steer did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteerOutcome {
    /// Handed to the running turn.
    Injected,
    /// The adapter started a new turn with it. A client that opts into
    /// `promptRequired` should not see this, but an adapter that ignores the
    /// opt-in may send it; the prompt is delivered either way.
    StartedNewTurn,
    /// No turn was running: the client must send the prompt itself.
    PromptRequired,
    /// The adapter could not steer (`codex-acp` answers `{"outcome":
    /// "failed"}` when its `turn/steer` errors). The prompt did not arrive.
    Failed,
}

impl SteerOutcome {
    /// The outcome a steer response reports, or `None` for one this does
    /// not know.
    pub fn from_response(response: &Value) -> Option<Self> {
        match response.get("outcome").and_then(Value::as_str)? {
            "injected" => Some(Self::Injected),
            "startedNewTurn" => Some(Self::StartedNewTurn),
            "promptRequired" => Some(Self::PromptRequired),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    /// Whether the prompt reached the agent, so it leaves the client's queue.
    pub fn delivered(self) -> bool {
        match self {
            Self::Injected | Self::StartedNewTurn => true,
            Self::PromptRequired | Self::Failed => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `claude-agent-acp` 0.101's `initialize` `_meta`, trimmed to the key.
    #[test]
    fn the_adapters_advertisement_is_recognised() {
        assert!(advertises_steering(Some(&json!({"steering": {"supported": true}}))));
        assert!(!advertises_steering(Some(&json!({"steering": {"supported": false}}))));
        assert!(!advertises_steering(Some(&json!({"claudeCode": {}}))));
        assert!(!advertises_steering(None));
    }

    #[test]
    fn the_request_opts_into_prompt_required() {
        let params = steer_params("s1", "also the tests");
        assert_eq!(params["sessionId"], "s1");
        assert_eq!(params["prompt"][0]["text"], "also the tests");
        assert_eq!(params["_meta"]["steering"]["idleBehavior"],
                   "promptRequired");
    }

    /// The three outcomes `steer()` returns, as it writes them.
    #[test]
    fn each_outcome_parses() {
        assert_eq!(SteerOutcome::from_response(&json!({"outcome": "injected"})),
                   Some(SteerOutcome::Injected));
        assert_eq!(SteerOutcome::from_response(&json!({"outcome": "startedNewTurn"})),
                   Some(SteerOutcome::StartedNewTurn));
        assert_eq!(SteerOutcome::from_response(&json!({"outcome": "promptRequired",
                                                       "reason": "noRunningTurn"})),
                   Some(SteerOutcome::PromptRequired));
        assert_eq!(SteerOutcome::from_response(&json!({"outcome": "failed"})),
                   Some(SteerOutcome::Failed));
        assert!(!SteerOutcome::Failed.delivered());
        assert_eq!(SteerOutcome::from_response(&json!({"outcome": "later"})),
                   None);
        assert!(SteerOutcome::Injected.delivered());
        assert!(!SteerOutcome::PromptRequired.delivered());
    }
}

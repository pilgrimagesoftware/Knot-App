//! Why an agent ended a prompt turn: the `stopReason` of the
//! `session/prompt` response
//! (<https://agentclientprotocol.com/protocol/prompt-turn>, "Stop Reasons").
//!
//! Kept rather than discarded (#22 follow-up): a turn the agent ended early -
//! out of tokens, out of model requests, refused - looks on screen exactly
//! like an agent that announced work and went idle, unless the reason is
//! shown.

use std::fmt;

/// An ACP stop reason, plus the one Knot synthesizes itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// `end_turn`: the agent finished.
    EndTurn,
    /// `max_tokens`: the model hit its output token limit.
    MaxTokens,
    /// `max_turn_requests`: the turn hit its limit of model requests.
    MaxTurnRequests,
    /// `refusal`: the agent refused to continue.
    Refusal,
    /// `cancelled`: the client cancelled the turn.
    Cancelled,
    /// Not an ACP value: Knot's own, for a prompt whose request failed or
    /// whose connection closed, so callers waiting on a turn end still get
    /// one. The failure itself is reported separately.
    Error,
    /// A value the spec does not name, kept verbatim so it can be shown.
    Other(String),
}

impl StopReason {
    /// The reason `value` names. A missing `stopReason` is `end_turn`, the
    /// spec's ordinary ending.
    pub fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("end_turn") {
            "end_turn" => Self::EndTurn,
            "max_tokens" => Self::MaxTokens,
            "max_turn_requests" => Self::MaxTurnRequests,
            "refusal" => Self::Refusal,
            "cancelled" => Self::Cancelled,
            "error" => Self::Error,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::EndTurn => "end_turn",
            Self::MaxTokens => "max_tokens",
            Self::MaxTurnRequests => "max_turn_requests",
            Self::Refusal => "refusal",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
            Self::Other(other) => other,
        }
    }
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every value the spec names, round-tripped.
    #[test]
    fn the_spec_values_parse_and_print_back() {
        for value in ["end_turn",
                      "max_tokens",
                      "max_turn_requests",
                      "refusal",
                      "cancelled"]
        {
            let reason = StopReason::parse(Some(value));
            assert!(!matches!(reason, StopReason::Other(_)), "{value}");
            assert_eq!(reason.as_str(), value);
        }
    }

    #[test]
    fn a_missing_reason_is_an_ordinary_end() {
        assert_eq!(StopReason::parse(None), StopReason::EndTurn);
    }

    #[test]
    fn an_unknown_reason_is_kept_verbatim() {
        assert_eq!(StopReason::parse(Some("quota")),
                   StopReason::Other("quota".into()));
        assert_eq!(StopReason::Other("quota".into()).to_string(), "quota");
    }
}

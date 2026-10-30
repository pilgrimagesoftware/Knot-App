//! The action matrix: what to do once a message is classified (or, for the
//! custom action, once the user's own prompt has answered).
//!
//! Pure functions - no agent id, no I/O, no settings lookup. The caller
//! (`knot-mcp-tools`'s hook handler) is the one that knows which agent this
//! is for and what queue an [`Effect`] lands in; keeping that out of this
//! crate is what makes `dispatch`/`dispatch_custom` testable with no mock
//! agent store at all.

use knot_core::AutopilotAction;

use crate::classify::{Classification, is_empty_response};

/// What Autopilot wants done, once a classification (or custom response) is
/// in hand. The caller turns this into agent-store + UI effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing to do - the agent is not waiting on anything, or the LLM
    /// call failed during classification (matches the Swift reference's
    /// `catch { return .completed }`, which never reaches `dispatchAction`).
    None,
    /// Mark the agent as needing input and notify, with no message of its
    /// own - this is also the custom action's failure fallback, matching
    /// the Swift reference's `catch { ... await markInput(...) }`.
    ///
    /// Carries the classification that flagged the reply, so the agent's
    /// cell can say why it is waiting (#344); `None` for the custom action's
    /// fallback, where nothing was classified.
    Mark(Option<Classification>),
    /// Show the decision dialog carrying the agent's last message.
    Ask {
        last_message:   String,
        classification: Classification,
    },
    /// Inject this text into the agent directly: `"yes, continue"` for an
    /// auto-continued binary question, or the custom action's LLM response.
    InjectText(String),
}

/// Dispatches a completed classification against the configured action.
/// The custom action is never passed here - see
/// [`crate::analyze`], which branches to [`dispatch_custom`] before
/// classification even runs, matching the Swift reference's `analyze`.
pub fn dispatch(action: AutopilotAction, classification: Classification, last_message: &str)
                -> Effect {
    if classification == Classification::Completed {
        return Effect::None;
    }
    match action {
        AutopilotAction::Mark => Effect::Mark(Some(classification)),
        AutopilotAction::Ask => Effect::Ask { last_message: last_message.to_string(),
                                              classification },
        AutopilotAction::Continue => {
            if classification == Classification::Binary {
                Effect::InjectText("yes, continue".to_string())
            }
            else {
                // Open questions can't be auto-answered - fall back to mark.
                Effect::Mark(Some(classification))
            }
        }
        AutopilotAction::Custom => Effect::None,
    }
}

/// Turns a custom-action LLM response into an effect: no-op for an empty
/// response or the `EMPTY` sentinel, otherwise inject the trimmed text.
pub fn dispatch_custom(response: &str) -> Effect {
    if is_empty_response(response) {
        Effect::None
    }
    else {
        Effect::InjectText(response.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_classification_never_dispatches() {
        for action in AutopilotAction::ALL.iter().copied() {
            assert_eq!(dispatch(action, Classification::Completed, "msg"),
                       Effect::None);
        }
    }

    #[test]
    fn mark_action_marks_regardless_of_classification() {
        assert_eq!(dispatch(AutopilotAction::Mark, Classification::Binary, "msg"),
                   Effect::Mark(Some(Classification::Binary)));
        assert_eq!(dispatch(AutopilotAction::Mark, Classification::Open, "msg"),
                   Effect::Mark(Some(Classification::Open)));
    }

    #[test]
    fn ask_action_carries_the_message_and_classification() {
        assert_eq!(dispatch(AutopilotAction::Ask, Classification::Open, "hello"),
                   Effect::Ask { last_message:   "hello".to_string(),
                                 classification: Classification::Open, });
    }

    #[test]
    fn continue_action_auto_answers_binary_only() {
        assert_eq!(dispatch(AutopilotAction::Continue, Classification::Binary, "msg"),
                   Effect::InjectText("yes, continue".to_string()));
        assert_eq!(dispatch(AutopilotAction::Continue, Classification::Open, "msg"),
                   Effect::Mark(Some(Classification::Open)));
    }

    #[test]
    fn custom_action_never_dispatches_through_dispatch() {
        assert_eq!(dispatch(AutopilotAction::Custom, Classification::Binary, "msg"),
                   Effect::None);
    }

    #[test]
    fn custom_response_empty_is_a_no_op() {
        assert_eq!(dispatch_custom(""), Effect::None);
        assert_eq!(dispatch_custom("  EMPTY  "), Effect::None);
    }

    #[test]
    fn custom_response_text_is_injected_trimmed() {
        assert_eq!(dispatch_custom("  use approach A  "),
                   Effect::InjectText("use approach A".to_string()));
    }
}

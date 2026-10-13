//! Autopilot: classifies an agent's last message and decides what to do
//! about it, when the agent has gone idle and may be waiting on the user.
//!
//! Ported from the Swift reference's `AutopilotService`. Contract:
//! `openspec/specs/autopilot/spec.md`; the trigger condition (which hook,
//! which agent types, when classification runs at all) is
//! `openspec/specs/agent-hooks/spec.md`'s "Autopilot classification
//! trigger" requirement - this crate is only what happens once that trigger
//! has already decided to classify.
//!
//! Split from the trigger deliberately: this crate knows nothing about
//! agent ids, hook payloads, or the GPUI queues the effect eventually lands
//! in - `analyze` takes a message and settings, and returns an [`Effect`]
//! the caller interprets. That is what makes `dispatch`/`dispatch_custom`
//! testable with no agent store, and what keeps the one blocking HTTP call
//! in this crate instead of duplicated per caller.

mod classify;
mod dispatch;
mod error;
mod llm;

pub use classify::{
    CLASSIFICATION_PROMPT, CUSTOM_PROMPT_PREFIX, Classification, is_empty_response,
    parse_classification,
};
pub use dispatch::{Effect, dispatch, dispatch_custom};
pub use error::AutopilotError;
use knot_core::{AiProvider, AutopilotAction};
pub use llm::ai_model_for;

/// Analyzes `last_message` and returns the effect to apply.
///
/// Blocking: makes one HTTP call to the configured provider. Callers run
/// this off the async runtime's worker threads (`spawn_blocking`), not
/// directly in an async context.
///
/// The custom action skips classification entirely, matching the Swift
/// reference's `analyze`: it sends the agent's message straight to the
/// user's own prompt and injects whatever comes back (or does nothing for
/// an empty/`EMPTY` response). Every other action classifies first and
/// consults [`dispatch`].
///
/// An empty `last_message` is already filtered by the caller
/// (`agent-hooks`'s trigger requirement); this function does not re-check.
pub fn analyze(last_message: &str, provider: AiProvider, api_key: &str, action: AutopilotAction,
               custom_prompt: &str)
               -> Effect {
    if action == AutopilotAction::Custom {
        return analyze_custom(last_message, provider, api_key, custom_prompt);
    }

    match classify(last_message, provider, api_key) {
        Some(classification) => dispatch(action, classification, last_message),
        // LLM failure during classification: the Swift reference's `catch`
        // returns `.completed`, which `analyze` then treats as "nothing to
        // do" rather than falling back to mark - a transient classifier
        // failure should not start paging the user on every idle turn.
        None => Effect::None,
    }
}

/// Classifies `message`, or `None` on any LLM failure (missing key,
/// unsupported provider, request error) - all three are deliberately
/// collapsed, matching the Swift reference swallowing the error and
/// returning `.completed`.
fn classify(message: &str, provider: AiProvider, api_key: &str) -> Option<Classification> {
    llm::complete(provider, api_key, CLASSIFICATION_PROMPT, message, Some(16))
        .ok()
        .map(|response| parse_classification(&response))
}

/// Runs the custom action: the user's prompt, prefixed with the fixed
/// system framing, against the agent's last message. An LLM failure here
/// falls back to [`Effect::Mark`], matching the Swift reference's
/// `analyzeCustom`'s `catch { await markInput(...) }` - unlike plain
/// classification failure, a user who configured a custom prompt still
/// wants to be told the agent is waiting.
fn analyze_custom(last_message: &str, provider: AiProvider, api_key: &str, custom_prompt: &str)
                  -> Effect {
    if custom_prompt.is_empty() {
        return Effect::None;
    }
    let system_prompt = format!("{CUSTOM_PROMPT_PREFIX}{custom_prompt}");
    match llm::complete(provider, api_key, &system_prompt, last_message, Some(1024)) {
        Ok(response) => dispatch_custom(&response),
        Err(_) => Effect::Mark,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_action_with_no_prompt_is_a_no_op() {
        assert_eq!(analyze("agent says hi",
                           AiProvider::OpenAi,
                           "key",
                           AutopilotAction::Custom,
                           ""),
                   Effect::None);
    }

    #[test]
    fn missing_api_key_never_marks_for_plain_classification() {
        // No network call can succeed with an empty key, so this exercises
        // the real failure path end to end rather than mocking `llm::complete`.
        assert_eq!(analyze("agent says hi",
                           AiProvider::OpenAi,
                           "",
                           AutopilotAction::Mark,
                           ""),
                   Effect::None);
    }

    #[test]
    fn missing_api_key_falls_back_to_mark_for_custom_action() {
        assert_eq!(analyze("agent says hi",
                           AiProvider::OpenAi,
                           "",
                           AutopilotAction::Custom,
                           "decide for me"),
                   Effect::Mark);
    }
}

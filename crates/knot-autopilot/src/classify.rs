//! Classification prompt, parsing, and the custom-action "no-op" sentinel.
//!
//! Ported from the Swift reference's `AutopilotService` (`InputClassification`,
//! `classificationPrompt`, `parseResponse`, `customPromptPrefix`,
//! `isEmptyResponse`).

/// What the classifier decided about an agent's last message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// The agent finished its work; no input is needed.
    Completed,
    /// The agent wants a yes/no confirmation to proceed.
    Binary,
    /// The agent asked an open-ended question.
    Open,
}

/// Parses a classifier response, defaulting to [`Classification::Completed`]
/// for anything that is not clearly "binary" or "open". Matches the Swift
/// reference's `parseResponse`: case-insensitive, prefix match.
pub fn parse_classification(response: &str) -> Classification {
    let trimmed = response.trim().to_lowercase();
    if trimmed == "binary" || trimmed.starts_with("binary") {
        Classification::Binary
    }
    else if trimmed == "open" || trimmed.starts_with("open") {
        Classification::Open
    }
    else {
        Classification::Completed
    }
}

/// Whether a custom-action LLM response signals "do nothing": empty,
/// whitespace-only, or the literal keyword `EMPTY` (case-insensitive).
pub fn is_empty_response(response: &str) -> bool {
    let trimmed = response.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("EMPTY")
}

/// System prompt that asks the model to classify the agent's last message
/// into exactly one of the three [`Classification`] categories.
pub const CLASSIFICATION_PROMPT: &str = r#"You are a classifier. Given the last message from an AI coding agent, classify it into exactly one of three categories.

Answer ONLY with one word: "completed", "binary", or "open".

- "completed": The agent has finished its work, is reporting results, or sharing a status update. No user input is needed.
- "binary": The agent has a clear plan and is asking for simple approval or confirmation to proceed. A yes/no answer would suffice.
- "open": The agent is asking an open-ended question, requesting the user to choose between options, or asking for information or feedback.

When in doubt between "binary" and "open", answer "open".
When in doubt between "completed" and anything else, answer "completed".

Examples of "completed":
- "I've completed the refactoring. Here's what I changed..."
- "The tests are all passing now."
- "I found the bug in line 42..."
- "Done! The feature is implemented."

Examples of "binary":
- "Should I proceed with this approach?"
- "Do you want me to implement this?"
- "Shall I continue?"
- "Is this plan okay?"
- "Ready to proceed. Should I go ahead?"
- "Should I reply to Ellie?"

Examples of "open":
- "Which option would you prefer?"
- "Do you want approach A or approach B?"
- "What should the API key be called?"
- "Here's the implementation plan. Let me know if you'd like changes."
- "Provide feedback on the plan before we go on."
- "What database should we use?""#;

/// System framing prepended to the user's custom autopilot prompt.
pub const CUSTOM_PROMPT_PREFIX: &str = r#"You are managing an AI coding agent. You will receive the agent's last message. Your response will be sent directly to the agent as input.

IMPORTANT: Reply with ONLY the exact text to send to the agent. No explanations, no reasoning, no preamble, no quotes — just the raw instruction for the agent. If no action is needed, reply with exactly "EMPTY" (nothing else).

GOOD: yes, continue
BAD: I think the agent should continue, so I'll reply with "yes, continue"

GOOD: use approach A with the factory pattern
BAD: Based on the agent's question, I recommend approach A. Here's what I'll tell it: "use approach A with the factory pattern"

GOOD: EMPTY
BAD: The agent has finished its work, no response needed. EMPTY

Additional instructions from the user:

"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_classifications_case_insensitively() {
        assert_eq!(parse_classification("binary"), Classification::Binary);
        assert_eq!(parse_classification("Binary."), Classification::Binary);
        assert_eq!(parse_classification("  OPEN  "), Classification::Open);
        assert_eq!(parse_classification("open-ended"), Classification::Open);
    }

    #[test]
    fn unknown_response_defaults_to_completed() {
        assert_eq!(parse_classification("completed"), Classification::Completed);
        assert_eq!(parse_classification("I don't know"),
                   Classification::Completed);
        assert_eq!(parse_classification(""), Classification::Completed);
    }

    #[test]
    fn empty_response_detection_matches_swift_reference() {
        assert!(is_empty_response(""));
        assert!(is_empty_response("   "));
        assert!(is_empty_response("EMPTY"));
        assert!(is_empty_response("  empty  "));
        assert!(!is_empty_response("yes, continue"));
    }
}

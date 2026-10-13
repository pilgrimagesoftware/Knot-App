use std::fmt;

/// Failures from calling the configured LLM provider.
///
/// Every call site in this crate treats these as recoverable: a classify
/// failure degrades to [`crate::Effect::None`] and a custom-action failure
/// degrades to [`crate::Effect::Mark`], matching the Swift reference's
/// `catch` blocks. Nothing here is surfaced to the agent.
#[derive(Debug)]
pub enum AutopilotError {
    MissingApiKey,
    UnsupportedProvider(knot_core::AiProvider),
    Request(String),
}

impl fmt::Display for AutopilotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingApiKey => f.write_str("AI API key is not configured"),
            Self::UnsupportedProvider(provider) => write!(f, "unsupported AI provider: {provider}"),
            Self::Request(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AutopilotError {}

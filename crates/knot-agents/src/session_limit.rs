//! A session stopped by its provider's usage limit (#318).
//!
//! The agent CLIs report it as the error that answers a prompt - Claude's
//! "You've hit your session limit · resets 5:20pm (America/Los_Angeles)",
//! Codex's "You've hit your usage limit" - so it is recognised from that
//! text. Nothing else distinguishes it from any other failed turn, and it is
//! the one failure the user can do nothing about but wait.

/// Phrases, lower-case, that mark an error as a usage limit.
const LIMIT_PHRASES: [&str; 2] = ["session limit", "usage limit"];

/// Where the reset time starts, lower-case.
const RESETS: &str = "resets ";

/// An agent's session has hit its usage limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLimit {
    /// When it lifts, as the provider said it - "5:20pm
    /// (America/Los_Angeles)" - or `None` when it did not say.
    pub resets: Option<String>,
}

impl SessionLimit {
    /// The limit `error` reports, if it reports one.
    #[must_use]
    pub fn from_error(error: &str) -> Option<Self> {
        // ASCII-only lowering keeps every byte offset where it was, so an
        // index found in `lower` slices `error` at the same place.
        let lower = error.to_ascii_lowercase();
        if !LIMIT_PHRASES.iter().any(|phrase| lower.contains(phrase)) {
            return None;
        }
        let resets = lower.find(RESETS)
                          .map(|at| error[at + RESETS.len()..].trim().trim_end_matches('.'))
                          .filter(|resets| !resets.is_empty())
                          .map(str::to_owned);
        Some(Self { resets })
    }
}

#[cfg(test)]
mod tests;

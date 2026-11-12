//! How long the agent's current turn has run and roughly how much it has
//! written, for the line beside the working indicator (#35) - Claude Code's
//! own "(42s · 1.2k tokens · esc to interrupt)".
//!
//! The elapsed time is exact: the panel knows when the turn started. The
//! token count is an estimate. ACP reports only the context window's
//! occupancy (`usage_update`), not a turn's output tokens, so this counts the
//! text the agent has streamed this turn at the usual four characters to a
//! token. Thinking is not streamed to the panel, so a turn that thinks a lot
//! reads low; the label says "~" for that reason.

use std::time::{Duration, SystemTime};

use super::PanelState;
use crate::consts::CHARS_PER_TOKEN;

/// The current turn's progress, while one is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnProgress {
    pub elapsed:       Duration,
    /// Characters streamed this turn over [`CHARS_PER_TOKEN`].
    pub approx_tokens: usize,
}

impl PanelState {
    /// The running turn's progress at `now`, or `None` with no turn running.
    pub fn turn_progress(&self, now: SystemTime) -> Option<TurnProgress> {
        if !self.turn_active {
            return None;
        }
        let started = self.turn_started_at?;
        Some(TurnProgress { elapsed:       now.duration_since(started).unwrap_or_default(),
                            approx_tokens: self.turn_text_chars.div_ceil(CHARS_PER_TOKEN), })
    }
}

impl TurnProgress {
    /// "42s · ~1.2k tokens" - or just the time before anything is written.
    pub fn label(self) -> String {
        let elapsed = format_elapsed(self.elapsed);
        if self.approx_tokens == 0 {
            return elapsed;
        }
        knot_core::l10n::t_with("panel.turn_progress",
                                &[("elapsed", &elapsed),
                                  ("tokens", &format_tokens(self.approx_tokens))])
    }
}

/// "42s", "3m 5s", "4m", "1h 2m" - the CLI's units, without zero-padding and
/// without a zero minor unit (#396). The label is in a proportional font, so
/// padding never kept its width steady anyway.
fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    let (major, minor) = match (seconds / 3600, (seconds % 3600) / 60, seconds % 60) {
        (0, 0, s) => return format!("{s}s"),
        (0, m, s) => (format!("{m}m"), (s > 0).then(|| format!("{s}s"))),
        (h, m, _) => (format!("{h}h"), (m > 0).then(|| format!("{m}m"))),
    };
    match minor {
        Some(minor) => format!("{major} {minor}"),
        None => major,
    }
}

/// "950", "1.2k", "34k" - a token count at a glance.
fn format_tokens(tokens: usize) -> String {
    match tokens {
        0..1000 => tokens.to_string(),
        1000..10_000 => format!("{:.1}k", tokens as f64 / 1000.0),
        _ => format!("{}k", tokens / 1000),
    }
}

#[cfg(test)]
mod tests;

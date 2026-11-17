//! What to do about a schedule right now: nothing, start, skip, or note a
//! missed run. A pure function of the schedule, the last run handled, the
//! clock and whether the agent is busy, so every rule in the spec's
//! scheduled-start requirements is testable without a window.

use jiff::{SignedDuration, Timestamp, Zoned};

use super::definition::Schedule;
use super::evaluate::previous_run;

/// How late a run may be and still start: the Mac was asleep, or Knot was
/// launching. Older runs are missed. See the spec's "Missed runs run at most
/// once, and only when recent".
pub const GRACE: SignedDuration = SignedDuration::from_mins(10);

/// Why a due run did not start: each is work a new conversation would
/// discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    TurnInProgress,
    PermissionPending,
    /// A question card is waiting for an answer (#430).
    QuestionPending,
    PromptQueued,
    /// It read a message and has not answered it (#405).
    OwesReply,
}

/// The answer for one agent on one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunDecision {
    /// No run has come due since the last one handled.
    NotDue,
    /// Start the agent for the run due at `run`.
    Start { run: Zoned },
    /// The run due at `run` is skipped, with the reason.
    Skip { run: Zoned, reason: SkipReason },
    /// The run due at `run` is too late to start.
    Missed { run: Zoned },
}

impl RunDecision {
    /// The run this decision handles, to record as handled so no later tick
    /// handles it again - including after a relaunch.
    #[must_use]
    pub fn handled(&self) -> Option<Timestamp> {
        match self {
            Self::NotDue => None,
            Self::Start { run } | Self::Skip { run, .. } | Self::Missed { run } => {
                Some(run.timestamp())
            }
        }
    }
}

/// The decision for `schedule` at `now`.
///
/// Only the latest due run is considered, so at most one run is handled per
/// agent per tick and missed runs are never replayed in a batch. A run at or
/// before `last_handled` has been dealt with already.
#[must_use]
/// `busy` is what would be discarded by starting now, if anything.
pub fn decide(schedule: &Schedule, last_handled: Option<Timestamp>, now: &Zoned,
              busy: Option<SkipReason>)
              -> RunDecision {
    let Some(run) = previous_run(schedule, now)
    else {
        return RunDecision::NotDue;
    };
    if last_handled.is_some_and(|handled| run.timestamp() <= handled) {
        return RunDecision::NotDue;
    }
    if now.timestamp().duration_since(run.timestamp()) > GRACE {
        return RunDecision::Missed { run };
    }
    if let Some(reason) = busy {
        return RunDecision::Skip { run, reason };
    }
    RunDecision::Start { run }
}

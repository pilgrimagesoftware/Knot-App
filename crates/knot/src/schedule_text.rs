//! The words for an agent's schedule and idle countdown (#404), built from
//! data the row snapshot already holds - pure, so the render path does no
//! I/O and the tests need no window.
//!
//! Contract: `openspec/changes/session-scheduling` - `agent-list-ui`'s "A
//! scheduled agent shows its next run" and "An idle countdown shows near
//! its end".

use std::time::Duration;

use jiff::Zoned;
use knot_core::l10n::{t, t_with};
use knot_core::schedule::{IdleAction, Schedule, SkipReason, next_run};

use crate::scheduled_starts::ScheduleOutcome;
use crate::workspace_window::idle::IdleCountdown;

/// How long a missed or skipped run is mentioned in place of the next one.
pub(crate) const OUTCOME_SHOWN_FOR: Duration = Duration::from_secs(60 * 60);

/// `run` as a reader wants it: the time alone today, or a day and a time.
pub(crate) fn when_text(run: &Zoned, now: &Zoned) -> String {
    let time = format!("{:02}:{:02}", run.hour(), run.minute());
    if run.date() == now.date() {
        return t_with("schedule.when_today", &[("time", &time)]);
    }
    let day = match run.weekday() {
        jiff::civil::Weekday::Monday => "mon",
        jiff::civil::Weekday::Tuesday => "tue",
        jiff::civil::Weekday::Wednesday => "wed",
        jiff::civil::Weekday::Thursday => "thu",
        jiff::civil::Weekday::Friday => "fri",
        jiff::civil::Weekday::Saturday => "sat",
        jiff::civil::Weekday::Sunday => "sun",
    };
    t_with("schedule.when_day",
           &[("day", &t(&format!("schedule.weekday_short.{day}"))),
             ("time", &time)])
}

/// The schedule's detail line: a recent missed or skipped run, else the
/// next run. `None` for no schedule, or one that cannot run.
pub(crate) fn schedule_line(schedule: Option<&Schedule>, outcome: Option<ScheduleOutcome>,
                            now: &Zoned)
                            -> Option<String> {
    let schedule = schedule.filter(|schedule| schedule.is_runnable())?;
    let recent = |at: jiff::Timestamp| {
        now.timestamp().duration_since(at).unsigned_abs() <= OUTCOME_SHOWN_FOR
    };
    let local = |at: jiff::Timestamp| at.to_zoned(now.time_zone().clone());
    match outcome {
        Some(ScheduleOutcome::Missed(at)) if recent(at) => {
            return Some(t_with("agent.schedule_missed",
                               &[("when", &when_text(&local(at), now))]));
        }
        Some(ScheduleOutcome::Skipped(at, reason)) if recent(at) => {
            return Some(t_with(skip_key(reason), &[("when", &when_text(&local(at), now))]));
        }
        _ => {}
    }
    let next = next_run(schedule, now)?;
    Some(t_with("agent.schedule_next", &[("when", &when_text(&next, now))]))
}

/// The catalog entry for a run skipped for `reason`.
pub(crate) fn skip_key(reason: SkipReason) -> &'static str {
    match reason {
        SkipReason::TurnInProgress => "agent.schedule_skipped_turn",
        SkipReason::PermissionPending => "agent.schedule_skipped_permission",
        SkipReason::QuestionPending => "agent.schedule_skipped_question",
        SkipReason::PromptQueued => "agent.schedule_skipped_queued",
        SkipReason::OwesReply => "agent.schedule_skipped_reply",
    }
}

/// The idle countdown's detail line.
pub(crate) fn idle_line(countdown: IdleCountdown) -> String {
    let key = match countdown.action {
        IdleAction::Deactivate => "agent.idle_deactivating",
        IdleAction::Reset => "agent.idle_resetting",
    };
    t_with(key,
           &[("idle", &countdown.idle_minutes.to_string()),
             ("left", &countdown.left_minutes.to_string())])
}

#[cfg(test)]
mod tests;

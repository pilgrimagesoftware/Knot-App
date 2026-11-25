//! When a schedule next comes due, through `croner` (ADR-0008).
//!
//! Every schedule - preset or cron - is evaluated as a cron pattern, so the
//! two share one evaluator and one set of daylight-saving rules. The DST
//! behavior the spec requires is pinned by this module's tests, not assumed
//! from `croner`.

use croner::Cron;
use croner::errors::CronError;
use croner::parser::{CronParser, Seconds, Year};
use jiff::ZonedDateTime;

use super::definition::{Preset, Schedule, WeekdaySet};

/// Parses standard five-field cron. Seconds and years are refused: `croner`
/// accepts six and seven fields by default, and a seconds field would let an
/// agent start every second.
pub(super) fn parse_cron(expression: &str) -> Result<Cron, CronError> {
    CronParser::builder().seconds(Seconds::Disallowed)
                         .year(Year::Disallowed)
                         .build()
                         .parse(expression)
}

/// The first run of `schedule` strictly after `after`, in `after`'s time
/// zone, or `None` for a schedule that cannot run.
#[must_use]
pub fn next_run(schedule: &Schedule, after: &ZonedDateTime) -> Option<ZonedDateTime> {
    let cron = cron_for(schedule)?;
    cron.find_next_occurrence(after, false).ok()
}

/// The latest run of `schedule` at or before `at`, or `None`.
#[must_use]
pub(super) fn previous_run(schedule: &Schedule, at: &ZonedDateTime) -> Option<ZonedDateTime> {
    let cron = cron_for(schedule)?;
    cron.find_previous_occurrence(at, true).ok()
}

fn cron_for(schedule: &Schedule) -> Option<Cron> {
    let pattern = match schedule {
        Schedule::Preset(preset) => preset_pattern(*preset),
        Schedule::Cron(expression) => expression.clone(),
        Schedule::Unreadable(_) => return None,
    };
    parse_cron(&pattern).ok()
}

/// The cron pattern a preset means.
fn preset_pattern(preset: Preset) -> String {
    match preset {
        Preset::Daily { at } => format!("{} {} * * *", at.minute(), at.hour()),
        Preset::Weekdays { at } => format!("{} {} * * 1-5", at.minute(), at.hour()),
        Preset::Days { days, at } => {
            format!("{} {} * * {}", at.minute(), at.hour(), cron_days(days))
        }
        Preset::EveryHours { hours } => format!("0 */{hours} * * *"),
    }
}

fn cron_days(days: WeekdaySet) -> String {
    days.days()
        .map(|day| day.cron_number().to_string())
        .collect::<Vec<_>>()
        .join(",")
}

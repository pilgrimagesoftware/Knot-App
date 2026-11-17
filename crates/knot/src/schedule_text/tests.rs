//! The schedule and idle-countdown lines, checked against the catalog's own
//! entries rather than English copy.

use jiff::Zoned;
use jiff::civil::date;
use jiff::tz::TimeZone;
use knot_core::l10n::{t, t_with};
use knot_core::schedule::{IdleAction, Schedule, SkipReason};

use super::{idle_line, schedule_line, when_text};
use crate::scheduled_starts::ScheduleOutcome;
use crate::workspace_window::idle::IdleCountdown;

/// Friday 2026-11-13.
fn la(day: i8, hour: i8, minute: i8) -> Zoned {
    date(2026, 11, day).at(hour, minute, 0, 0)
                       .to_zoned(TimeZone::get("America/Los_Angeles").expect("tzdb has LA"))
                       .expect("a valid local time")
}

fn schedule(text: &str) -> Schedule {
    text.parse().unwrap_or_else(|()| panic!("{text} parses"))
}

#[test]
fn a_run_today_reads_as_its_time_and_another_day_with_its_day() {
    let now = la(13, 8, 0);
    assert_eq!(when_text(&la(13, 9, 5), &now),
               t_with("schedule.when_today", &[("time", "09:05")]));
    assert_eq!(when_text(&la(16, 9, 0), &now),
               t_with("schedule.when_day",
                      &[("day", &t("schedule.weekday_short.mon")), ("time", "09:00")]));
}

#[test]
fn every_weekday_name_resolves() {
    for day in ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] {
        let key = format!("schedule.weekday_short.{day}");
        assert_ne!(t(&key), key, "{key} is missing from the catalog");
    }
}

#[test]
fn a_scheduled_agent_shows_its_next_run() {
    let now = la(13, 18, 0);
    let line = schedule_line(Some(&schedule("weekdays 09:00")), None, &now).expect("a line");
    assert_eq!(line,
               t_with("agent.schedule_next",
                      &[("when", &when_text(&la(16, 9, 0), &now))]));
}

#[test]
fn a_recent_missed_run_is_named_and_an_old_one_gives_way_to_the_next() {
    let missed = Some(ScheduleOutcome::Missed(la(13, 9, 0).timestamp()));
    let daily = schedule("daily 09:00");

    let soon_after = schedule_line(Some(&daily), missed, &la(13, 9, 40)).expect("a line");
    assert_eq!(soon_after,
               t_with("agent.schedule_missed",
                      &[("when", &when_text(&la(13, 9, 0), &la(13, 9, 40)))]));

    let much_later = schedule_line(Some(&daily), missed, &la(13, 12, 0)).expect("a line");
    assert_eq!(much_later,
               t_with("agent.schedule_next",
                      &[("when", &when_text(&la(14, 9, 0), &la(13, 12, 0)))]));
}

#[test]
fn a_recent_skipped_run_is_named() {
    let skipped = Some(ScheduleOutcome::Skipped(la(13, 9, 0).timestamp(), SkipReason::OwesReply));
    let line =
        schedule_line(Some(&schedule("daily 09:00")), skipped, &la(13, 9, 10)).expect("a line");
    assert_eq!(line,
               t_with("agent.schedule_skipped_reply",
                      &[("when", &when_text(&la(13, 9, 0), &la(13, 9, 10)))]));
}

#[test]
fn no_schedule_or_an_unreadable_one_shows_nothing() {
    let now = la(13, 8, 0);
    assert_eq!(schedule_line(None, None, &now), None);
    assert_eq!(schedule_line(Some(&Schedule::Unreadable("teatime".into())), None, &now),
               None);
}

#[test]
fn the_countdown_names_both_durations_and_the_action() {
    for (action, key) in [(IdleAction::Deactivate, "agent.idle_deactivating"),
                          (IdleAction::Reset, "agent.idle_resetting")]
    {
        let line = idle_line(IdleCountdown { idle_minutes: 52,
                                             left_minutes: 8,
                                             action });
        assert_eq!(line, t_with(key, &[("idle", "52"), ("left", "8")]));
        assert!(line.contains("52") && line.contains('8'),
                "values survive: {line}");
    }
}

#[test]
fn every_skip_reason_has_its_own_line() {
    use super::skip_key;
    let keys = [SkipReason::TurnInProgress,
                SkipReason::PermissionPending,
                SkipReason::QuestionPending,
                SkipReason::PromptQueued,
                SkipReason::OwesReply].map(skip_key);
    for key in keys {
        assert_ne!(t(key), key, "{key} is missing from the catalog");
    }
    let distinct: std::collections::BTreeSet<_> = keys.iter().map(|key| t(key)).collect();
    assert_eq!(distinct.len(), keys.len(), "each reason reads differently");
}

/// A skipped line resolves to real text, not the key: a vanished key would
/// make the comparison above pass vacuously.
#[test]
fn the_skipped_line_is_real_text() {
    let skipped = Some(ScheduleOutcome::Skipped(la(13, 9, 0).timestamp(), SkipReason::OwesReply));
    let line =
        schedule_line(Some(&schedule("daily 09:00")), skipped, &la(13, 9, 10)).expect("a line");
    assert!(!line.starts_with("agent."), "unresolved key: {line}");
}

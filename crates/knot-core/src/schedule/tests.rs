//! The schedule vocabulary, `next_run` through `croner`, and the run
//! decision. The DST cases pin what the spec requires of `croner` (ADR-0008):
//! a skipped local time runs at the first valid time after it, and a repeated
//! one runs once.

use jiff::civil::date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, ZonedDateTime};

use super::*;

fn la(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> ZonedDateTime {
    date(year, month, day).at(hour, minute, 0, 0)
                          .to_zoned_datetime(TimeZone::get("America/Los_Angeles").expect("tzdb has LA"))
                          .expect("a valid local time")
}

fn at(hour: u8, minute: u8) -> TimeOfDay {
    TimeOfDay::new(hour, minute).expect("a valid time")
}

fn schedule(text: &str) -> Schedule {
    text.parse().unwrap_or_else(|()| panic!("{text} parses"))
}

fn hm(zoned: &ZonedDateTime) -> (i8, i8) {
    (zoned.hour(), zoned.minute())
}

#[test]
fn every_preset_round_trips_through_its_stored_text() {
    let days = WeekdaySet::new([Weekday::Monday, Weekday::Wednesday]).expect("non-empty");
    for preset in [Preset::Daily { at: at(9, 0) },
                   Preset::Weekdays { at: at(8, 30) },
                   Preset::Days { days,
                                  at: at(17, 5) },
                   Preset::EveryHours { hours: 5 }]
    {
        let original = Schedule::Preset(preset);
        assert_eq!(schedule(&original.to_string()), original);
    }
    assert_eq!(Schedule::Preset(Preset::Days { days,
                                               at: at(17, 5) }).to_string(),
               "days mon,wed 17:05");
}

#[test]
fn a_cron_schedule_round_trips_and_is_normalized() {
    let cron = Schedule::cron("30  8 * *  1-5").expect("valid five-field cron");
    assert_eq!(cron.to_string(), "cron 30 8 * * 1-5");
    assert_eq!(schedule("cron 30 8 * * 1-5"), cron);
}

#[test]
fn only_five_field_cron_is_accepted() {
    assert!(Schedule::cron("0 9 * * 1-5").is_some());
    assert!(Schedule::cron("* * * * * *").is_none(),
            "a seconds field would run every second");
    assert!(Schedule::cron("0 0 9 * * 1-5 2027").is_none());
    assert!(Schedule::cron("61 9 * * *").is_none());
    assert!(Schedule::cron("not cron").is_none());
}

/// croner 5 (jiff 1.0) reads names only in their own field and refuses a
/// bare `L` for a weekday. Both used to parse and run on an unintended
/// schedule - `jan` as Monday, `L` as nothing at all - so refusing them is
/// the fix, not a regression.
#[test]
fn a_name_outside_its_field_and_a_bare_weekday_l_are_refused() {
    assert!(Schedule::cron("0 0 * * jan").is_none(),
            "a month name in the weekday field");
    assert!(Schedule::cron("mon 0 * * *").is_none(),
            "a weekday name in the minute field");
    assert!(Schedule::cron("0 0 * * L").is_none(),
            "a bare L for a weekday");
    assert!(Schedule::cron("0 0 * jan mon").is_some(),
            "names in their own fields still parse");
}

/// A stored pattern croner 5 refuses loads as unreadable and never runs,
/// rather than failing the document or running on a guess.
#[test]
fn a_stored_pattern_croner_now_refuses_never_runs() {
    let loaded: Schedule = serde_json::from_str("\"cron 0 0 * * jan\"").expect("never fails");
    assert!(!loaded.is_runnable());
    assert_eq!(next_run(&loaded, &la(2026, 11, 16, 9, 0)), None);
}

/// `SUN-SUN` is Sundays only (croner 5); it used to read as every day.
#[test]
fn a_sunday_to_sunday_range_is_sundays_only() {
    // 2026-11-16 is a Monday; the next Sunday is the 22nd.
    let next = next_run(&schedule("cron 0 9 * * SUN-SUN"), &la(2026, 11, 16, 8, 0)).expect("runs");
    assert_eq!((next.day(), hm(&next)), (22, (9, 0)));
}

#[test]
fn an_unreadable_stored_schedule_is_kept_and_never_runs() {
    let loaded: Schedule = serde_json::from_str("\"fortnightly at teatime\"").expect("never fails");
    assert_eq!(loaded,
               Schedule::Unreadable("fortnightly at teatime".to_owned()));
    assert!(!loaded.is_runnable());
    assert_eq!(serde_json::to_string(&loaded).expect("serializes"),
               "\"fortnightly at teatime\"");
    assert_eq!(next_run(&loaded, &la(2026, 11, 16, 9, 0)), None);
}

#[test]
fn out_of_range_presets_do_not_parse() {
    for text in ["daily 24:00",
                 "daily 9:60",
                 "every 0h",
                 "every 24h",
                 "days 09:00",
                 "days xyz 09:00"]
    {
        assert!(text.parse::<Schedule>().is_err(), "{text} should not parse");
    }
}

#[test]
fn a_daily_run_comes_next_at_its_time() {
    let next = next_run(&schedule("daily 09:00"), &la(2026, 11, 16, 8, 0)).expect("runs");
    assert_eq!((next.day(), hm(&next)), (16, (9, 0)));
    let next = next_run(&schedule("daily 09:00"), &la(2026, 11, 16, 9, 0)).expect("runs");
    assert_eq!(next.day(),
               17,
               "strictly after: a run at 09:00 is not next at 09:00");
}

#[test]
fn weekdays_skip_the_weekend() {
    // 2026-11-13 is a Friday.
    let next = next_run(&schedule("weekdays 09:00"), &la(2026, 11, 13, 10, 0)).expect("runs");
    assert_eq!((next.day(), hm(&next)),
               (16, (9, 0)),
               "Friday evening -> Monday 09:00");
}

#[test]
fn chosen_days_run_only_on_those_days() {
    // Monday 2026-11-16, after 09:00: next is Wednesday.
    let next = next_run(&schedule("days mon,wed 09:00"), &la(2026, 11, 16, 10, 0)).expect("runs");
    assert_eq!(next.day(), 18);
}

#[test]
fn every_n_hours_runs_at_multiples_from_midnight() {
    let every5 = schedule("every 5h");
    let mut at = la(2026, 11, 16, 0, 30);
    let mut hours = Vec::new();
    for _ in 0..6 {
        at = next_run(&every5, &at).expect("runs");
        hours.push((at.day(), at.hour()));
    }
    // 5, 10, 15, 20, then midnight: the last gap of the day is 4 hours.
    assert_eq!(hours,
               [(16, 5), (16, 10), (16, 15), (16, 20), (17, 0), (17, 5)]);
}

#[test]
fn a_cron_expression_runs_on_its_weekdays() {
    let cron = Schedule::cron("30 8 * * 1-5").expect("valid");
    // Saturday 2026-11-14 -> Monday 08:30.
    let next = next_run(&cron, &la(2026, 11, 14, 12, 0)).expect("runs");
    assert_eq!((next.day(), hm(&next)), (16, (8, 30)));
}

/// 2026-03-08: Los Angeles springs forward from 02:00 to 03:00.
#[test]
fn a_skipped_local_time_runs_once_at_the_first_valid_time_after_it() {
    let next = next_run(&schedule("daily 02:30"), &la(2026, 3, 8, 1, 0)).expect("runs");
    assert_eq!((next.day(), hm(&next)), (8, (3, 0)));
    let after = next_run(&schedule("daily 02:30"), &next).expect("runs");
    assert_eq!((after.day(), hm(&after)), (9, (2, 30)), "and once only");
}

/// 2026-11-01: Los Angeles falls back from 02:00 to 01:00, so 01:30 happens
/// twice.
#[test]
fn a_repeated_local_time_runs_once() {
    let daily = schedule("daily 01:30");
    let first = next_run(&daily, &la(2026, 11, 1, 0, 0)).expect("runs");
    assert_eq!((first.day(), hm(&first)), (1, (1, 30)));
    let second = next_run(&daily, &first).expect("runs");
    assert_eq!(second.day(),
               2,
               "the second 01:30 of the night does not run again");
}

#[test]
fn a_due_run_starts_once() {
    let daily = schedule("daily 09:00");
    let now = la(2026, 11, 16, 9, 1);
    let decision = decide(&daily, None, &now, None);
    let RunDecision::Start { run } = &decision
    else {
        panic!("expected a start, got {decision:?}");
    };
    assert_eq!(hm(run), (9, 0));
    assert_eq!(decide(&daily, decision.handled(), &now, None),
               RunDecision::NotDue);
}

#[test]
fn a_relaunch_inside_the_grace_window_does_not_start_the_run_again() {
    let daily = schedule("daily 09:00");
    let handled = decide(&daily, None, &la(2026, 11, 16, 9, 0), None).handled();
    // Knot quits and relaunches at 09:05; the handled run was persisted.
    assert_eq!(decide(&daily, handled, &la(2026, 11, 16, 9, 5), None),
               RunDecision::NotDue);
}

#[test]
fn a_busy_agent_skips_the_run_and_the_skip_is_final() {
    let daily = schedule("daily 09:00");
    let decision = decide(&daily,
                          None,
                          &la(2026, 11, 16, 9, 0),
                          Some(SkipReason::TurnInProgress));
    assert!(matches!(decision,
                     RunDecision::Skip { reason: SkipReason::TurnInProgress,
                                         .. }));
    // Free again at 09:03: still inside the grace window, but already handled.
    assert_eq!(decide(&daily, decision.handled(), &la(2026, 11, 16, 9, 3), None),
               RunDecision::NotDue);
}

#[test]
fn a_run_up_to_the_grace_period_late_still_starts() {
    let daily = schedule("daily 09:00");
    let late = la(2026, 11, 16, 9, 0).checked_add(GRACE).expect("in range");
    assert!(matches!(decide(&daily, None, &late, None), RunDecision::Start { .. }));
}

#[test]
fn an_older_run_is_missed_and_only_the_latest_counts() {
    let every3 = schedule("every 3h");
    // Last handled at 06:00; Knot is back at 13:00. 09:00 and 12:00 came due.
    let handled = Some(la(2026, 11, 16, 6, 0).timestamp());
    let decision = decide(&every3, handled, &la(2026, 11, 16, 13, 0), None);
    let RunDecision::Missed { run } = &decision
    else {
        panic!("expected a missed run, got {decision:?}");
    };
    assert_eq!(hm(run), (12, 0), "only the latest due run is handled");
    assert_eq!(decide(&every3, decision.handled(), &la(2026, 11, 16, 13, 1), None),
               RunDecision::NotDue);
}

#[test]
fn idle_policies_round_trip_and_resolve_against_the_default() {
    let rule = IdleRule::new(30, IdleAction::Deactivate).expect("non-zero");
    for policy in [IdlePolicy::Inherit,
                   IdlePolicy::Off,
                   IdlePolicy::Custom(rule)]
    {
        assert_eq!(policy.to_string().parse::<IdlePolicy>(), Ok(policy));
    }
    let default = IdleRule::new(60, IdleAction::Reset);
    assert_eq!(IdlePolicy::Inherit.effective(default), default);
    assert_eq!(IdlePolicy::Off.effective(default), None);
    assert_eq!(IdlePolicy::Custom(rule).effective(default), Some(rule));
}

#[test]
fn an_unreadable_idle_policy_loads_as_inherit() {
    let loaded: IdlePolicy = serde_json::from_str("\"nap 5 minutes\"").expect("never fails");
    assert_eq!(loaded, IdlePolicy::Inherit);
    assert!(IdleRule::new(0, IdleAction::Reset).is_none());
}

#[test]
fn the_grace_period_is_ten_minutes() {
    assert_eq!(GRACE, SignedDuration::from_mins(10));
}

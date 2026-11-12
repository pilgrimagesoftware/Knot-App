//! Recognising a usage limit from the error that answers a prompt (#318).

use super::SessionLimit;

/// The error from the issue's screenshot, verbatim.
#[test]
fn claudes_session_limit_is_recognised_with_its_reset_time() {
    let error = "The agent could not answer: acp error response (code -32603): Internal \
                 error: You've hit your session limit · resets 5:20pm (America/Los_Angeles)";

    assert_eq!(SessionLimit::from_error(error),
               Some(SessionLimit { resets: Some("5:20pm (America/Los_Angeles)".to_owned()), }));
}

#[test]
fn a_usage_limit_without_a_reset_time_is_still_a_limit() {
    assert_eq!(SessionLimit::from_error("You've hit your usage limit."),
               Some(SessionLimit { resets: None }));
}

/// #397: Claude now names the limit it hit. The wording the SDK builds is
/// "You've hit your {name}" with the name from its own table, so each name
/// it prints today is a limit, and so is the older "session limit" above.
#[test]
fn every_limit_claude_names_is_recognised_with_its_reset_time() {
    for name in ["5-hour limit",
                 "weekly limit",
                 "Opus limit",
                 "Sonnet limit",
                 "usage credit limit"]
    {
        let error = format!("acp error response (code -32603): Internal error: You've hit your \
                             {name} · resets 5:20pm (America/Los_Angeles)");
        assert_eq!(SessionLimit::from_error(&error),
                   Some(SessionLimit { resets: Some("5:20pm (America/Los_Angeles)".to_owned()), }),
                   "{name}");
    }
}

/// The SDK's other stable lead, with a typographic apostrophe and no reset.
#[test]
fn a_limit_reached_rather_than_hit_is_a_limit() {
    assert_eq!(SessionLimit::from_error("You\u{2019}ve reached your Fable limit."),
               Some(SessionLimit { resets: None }));
}

/// The same lead on something that does not stop the session: fast mode's
/// cooldown falls back to normal speed, and a spend budget is not a limit
/// that lifts on its own.
#[test]
fn a_lead_that_names_no_stopping_limit_is_not_a_limit() {
    for error in ["You've hit your fast limit · resets in 4m",
                  "You've hit your team's shared budget. /model to switch models.",
                  "You've hit your stride, and the tests now pass the rate check"]
    {
        assert_eq!(SessionLimit::from_error(error), None, "{error}");
    }
}

#[test]
fn a_line_opening_with_a_named_limit_is_a_notice() {
    assert!(SessionLimit::opens_with_named_limit("you've hit your 5-hour limit · resets 5pm"));
    assert!(SessionLimit::opens_with_named_limit("you\u{2019}ve hit your weekly limit"));
    assert!(!SessionLimit::opens_with_named_limit("note: you've hit your 5-hour limit"),
            "only a line's start is a notice");
    assert!(!SessionLimit::opens_with_named_limit("you've hit your fast limit · resets in 4m"));
}

#[test]
fn any_other_error_is_not_a_limit() {
    for error in ["quota exhausted",
                  "acp error response (code -32603): Internal error",
                  "connection reset by peer"]
    {
        assert_eq!(SessionLimit::from_error(error), None, "{error}");
    }
}

/// 2026-11-04 12:00 UTC: 04:00 in Los Angeles, 13:00 in Paris.
fn refused_at() -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_793_793_600)
}

fn limit(resets: &str) -> SessionLimit {
    SessionLimit { resets: Some(resets.to_owned()), }
}

fn utc(seconds: u64) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds)
}

/// #368: the reset is the next time the named zone's clock reads it.
#[test]
fn a_reset_later_the_same_day_lifts_that_day() {
    // 17:20 in Los Angeles (PST, UTC-8) on 2026-11-04 is 01:20 UTC on the 5th.
    assert_eq!(limit("5:20pm (America/Los_Angeles)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 13 * 3600 + 20 * 60)));
}

#[test]
fn a_reset_already_past_today_lifts_tomorrow() {
    // 09:00 in Paris has passed at 13:00 Paris time, so it is 09:00 on the
    // 5th: 08:00 UTC (CET, UTC+1).
    assert_eq!(limit("9am (Europe/Paris)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 20 * 3600)));
}

#[test]
fn twelve_oclock_is_noon_or_midnight() {
    // 12pm in Paris is 11:00 UTC on the 5th (passed today); 12am is 23:00
    // UTC on the 4th.
    assert_eq!(limit("12pm (Europe/Paris)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 23 * 3600)));
    assert_eq!(limit("12am (Europe/Paris)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 11 * 3600)));
}

/// A time this cannot be sure of is no time to send a held prompt at.
#[test]
fn a_reset_it_cannot_read_has_no_time() {
    for resets in ["in 3 hours",
                   "5:20pm",
                   "5:20pm (Not/AZone)",
                   "Nov 5, 9am",
                   "25pm (Europe/Paris)",
                   "5:75pm (Europe/Paris)"]
    {
        assert_eq!(limit(resets).lifts_at(refused_at()), None, "{resets}");
    }
    assert_eq!(SessionLimit { resets: None }.lifts_at(refused_at()), None);
}

/// #368 review: a provider whose clock runs behind refuses a prompt sent
/// just after the reset with the same reset time. That is the reset due now,
/// so it retries in a minute - not at once, which would loop, and not
/// tomorrow, which would hold the prompt for a day.
#[test]
fn a_reset_only_just_past_retries_shortly() {
    // Refused at 13:00:30 in Paris; the 12:59 reset was ninety seconds ago.
    let refused = refused_at();
    let late = limit("12:59pm (Europe/Paris)").lifts_at(refused
                                                        + std::time::Duration::from_secs(30))
                                              .expect("a reset time");
    assert_eq!(late,
               refused + std::time::Duration::from_secs(30 + 60),
               "a minute after the refusal");
}

/// Further past than the grace, it is tomorrow's reset.
#[test]
fn a_reset_well_past_is_tomorrows() {
    // 13:00 in Paris; 12:40 was twenty minutes ago, so it is 12:40 on the
    // 5th: 11:40 UTC.
    assert_eq!(limit("12:40pm (Europe/Paris)").lifts_at(refused_at()),
               Some(utc(1_793_793_600 + 23 * 3600 + 40 * 60)));
}

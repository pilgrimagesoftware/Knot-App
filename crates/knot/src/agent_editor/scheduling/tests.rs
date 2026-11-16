//! What the scheduling controls submit, as pure functions of their values.

use knot_core::schedule::{IdleAction, IdlePolicy, IdleRule, Schedule};

use super::{IdleKind, ScheduleKind, ScheduleValues, idle_policy_from, schedule_from};

fn values(kind: ScheduleKind) -> ScheduleValues<'static> {
    ScheduleValues { kind,
                     time: "09:00",
                     hours: "3",
                     cron: "0 9 * * 1-5",
                     days: [true, false, true, false, false, false, false] }
}

#[test]
fn each_kind_submits_its_schedule() {
    let submitted = |kind| {
        schedule_from(values(kind)).expect("valid")
                                   .map(|s| s.to_string())
    };
    assert_eq!(submitted(ScheduleKind::None), None);
    assert_eq!(submitted(ScheduleKind::Daily).as_deref(),
               Some("daily 09:00"));
    assert_eq!(submitted(ScheduleKind::Weekdays).as_deref(),
               Some("weekdays 09:00"));
    assert_eq!(submitted(ScheduleKind::Days).as_deref(),
               Some("days mon,wed 09:00"));
    assert_eq!(submitted(ScheduleKind::EveryHours).as_deref(),
               Some("every 3h"));
    assert_eq!(submitted(ScheduleKind::Cron).as_deref(),
               Some("cron 0 9 * * 1-5"));
}

#[test]
fn each_bad_entry_names_what_to_fix() {
    let error = |values| schedule_from(values).expect_err("invalid");
    assert_eq!(error(ScheduleValues { time: "9am",
                                      ..values(ScheduleKind::Daily) }),
               "agent_editor.schedule_error_time");
    assert_eq!(error(ScheduleValues { days: [false; 7],
                                      ..values(ScheduleKind::Days) }),
               "agent_editor.schedule_error_days");
    assert_eq!(error(ScheduleValues { hours: "24",
                                      ..values(ScheduleKind::EveryHours) }),
               "agent_editor.schedule_error_hours");
    assert_eq!(error(ScheduleValues { cron: "* * * * * *",
                                      ..values(ScheduleKind::Cron) }),
               "agent_editor.schedule_error_cron");
}

#[test]
fn every_error_key_resolves() {
    for key in ["agent_editor.schedule_error_time",
                "agent_editor.schedule_error_days",
                "agent_editor.schedule_error_hours",
                "agent_editor.schedule_error_cron",
                "agent_editor.schedule_error_no_prompt",
                "agent_editor.idle_error_minutes"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn the_idle_controls_submit_a_policy() {
    assert_eq!(idle_policy_from(IdleKind::Inherit, ""),
               Ok(IdlePolicy::Inherit));
    assert_eq!(idle_policy_from(IdleKind::Off, "junk"), Ok(IdlePolicy::Off));
    assert_eq!(idle_policy_from(IdleKind::Reset, " 45 "),
               Ok(IdlePolicy::Custom(IdleRule::new(45, IdleAction::Reset).expect("non-zero"))));
    assert_eq!(idle_policy_from(IdleKind::Deactivate, "0"),
               Err("agent_editor.idle_error_minutes"));
}

#[test]
fn a_cron_submission_round_trips_through_the_store_text() {
    let cron = schedule_from(values(ScheduleKind::Cron)).expect("valid")
                                                        .expect("some");
    assert_eq!(cron, Schedule::cron("0 9 * * 1-5").expect("valid"));
}

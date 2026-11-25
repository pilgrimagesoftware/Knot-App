//! An agent's schedule and idle policy (#404): set by the editor through
//! `edit`, or directly, without restarting a running agent, and every change
//! moves the revision so open windows redraw the row.

use knot_core::schedule::{IdleAction, IdlePolicy, IdleRule, Schedule};

use crate::{AgentState, AgentStore, CreateOptions, EditRequest};

fn running_agent(store: &mut AgentStore) -> uuid::Uuid {
    let id = store.create("/repo", CreateOptions::default());
    store.agent_mut(id).unwrap().state = AgentState::Running;
    id
}

#[test]
fn editing_the_schedule_and_idle_policy_does_not_restart() {
    let mut store = AgentStore::default();
    let id = running_agent(&mut store);
    let agent = store.agent(id).unwrap();
    let token = agent.restart_token;
    let schedule: Schedule = "weekdays 09:00".parse().expect("a preset");
    let policy = IdlePolicy::Custom(IdleRule::new(30, IdleAction::Reset).expect("non-zero"));
    let req = EditRequest { name: agent.name.clone(),
                            avatar: agent.avatar.clone(),
                            folder: Some(agent.folder.clone()),
                            agent_type: Some(agent.agent_type.clone()),
                            schedule: Some(schedule.clone()),
                            idle_policy: policy,
                            ..Default::default() };

    store.edit(id, req).expect("edit succeeds");

    let agent = store.agent(id).unwrap();
    assert_eq!(agent.restart_token, token,
               "a schedule acts at its next run, not now");
    assert_eq!(agent.schedule, Some(schedule));
    assert_eq!(agent.idle_policy, policy);
}

#[test]
fn setting_a_schedule_moves_the_revision_only_when_it_changes() {
    let mut store = AgentStore::default();
    let id = running_agent(&mut store);
    let before = store.revision();
    store.set_schedule(id, Some("daily 09:00".parse().expect("a preset")));
    let after = store.revision();
    assert_ne!(after, before, "the row's clock line has to redraw");
    store.set_schedule(id, Some("daily 09:00".parse().expect("a preset")));
    assert_eq!(store.revision(), after, "no change, no redraw");
    store.set_idle_policy(id, IdlePolicy::Off);
    assert_ne!(store.revision(), after);
}

fn la(day: i8, hour: i8, minute: i8) -> jiff::ZonedDateTime {
    jiff::civil::date(2026, 11, day).at(hour, minute, 0, 0)
                                    .to_zoned_datetime(jiff::tz::TimeZone::get("America/Los_Angeles")
                                                  .expect("tzdb has LA"))
                                    .expect("a valid local time")
}

fn handled(store: &AgentStore, id: uuid::Uuid) -> Option<jiff::Timestamp> {
    let workspace = store.workspaces()
                         .iter()
                         .find(|workspace| workspace.agent_ids.contains(&id))
                         .expect("in a workspace")
                         .id;
    store.schedule_run_handled(workspace, id)
         .and_then(|seconds| jiff::Timestamp::from_second(seconds).ok())
}

fn agent_in_a_workspace(store: &mut AgentStore) -> uuid::Uuid {
    let workspace = knot_core::Workspace { id:        uuid::Uuid::new_v4(),
                                           name:      "Scheduled".to_string(),
                                           color_hex: "#000000".to_string(),
                                           agent_ids: Vec::new(), };
    let workspace_id = workspace.id;
    store.add_workspace(workspace);
    store.create("/repo",
                 CreateOptions { workspace_id: Some(workspace_id),
                                 ..Default::default() })
}

/// The review case for #404: "daily 09:00" saved at 09:05 must not start the
/// agent (and reset its conversation) at once; it waits for tomorrow.
#[test]
fn a_schedule_set_after_its_time_waits_for_the_next_run() {
    use knot_core::schedule::{RunDecision, decide};

    let mut store = AgentStore::default();
    let id = agent_in_a_workspace(&mut store);
    let schedule: Schedule = "daily 09:00".parse().expect("a preset");
    store.set_schedule_at(id, Some(schedule.clone()), la(16, 9, 5).timestamp());

    assert_eq!(decide(&schedule, handled(&store, id), &la(16, 9, 6), None),
               RunDecision::NotDue,
               "the 09:00 run fell due before the schedule existed");
    assert!(matches!(decide(&schedule, handled(&store, id), &la(17, 9, 0), None),
                     RunDecision::Start { .. }),
            "the next occurrence runs");
}

#[test]
fn saving_an_unchanged_schedule_does_not_reset_what_was_handled() {
    let mut store = AgentStore::default();
    let id = agent_in_a_workspace(&mut store);
    let schedule: Schedule = "daily 09:00".parse().expect("a preset");
    store.set_schedule_at(id, Some(schedule.clone()), la(16, 8, 0).timestamp());
    let first = handled(&store, id);

    store.set_schedule_at(id, Some(schedule), la(16, 9, 5).timestamp());

    assert_eq!(handled(&store, id), first);
}

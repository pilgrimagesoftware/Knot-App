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

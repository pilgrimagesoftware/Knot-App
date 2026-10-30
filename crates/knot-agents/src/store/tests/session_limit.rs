//! `Agent::session_limit`: the usage limit that stopped an agent, kept
//! exactly as long as the agent shows it (#318).

use super::super::*;
use crate::SessionLimit;

fn limit() -> SessionLimit {
    SessionLimit { resets: Some("5:20pm".to_owned()), }
}

#[test]
fn a_limit_is_shown_while_the_agent_is_in_error() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());

    store.set_session_limit(id, Some(limit()));
    assert_eq!(store.agent(id).unwrap().active_session_limit(),
               None,
               "not shown until the agent is in Error");
    store.set_state(id, AgentState::Error);

    assert_eq!(store.agent(id).unwrap().active_session_limit(),
               Some(&limit()));
}

/// The next prompt moves the agent on, and the limit goes with it.
#[test]
fn leaving_error_clears_the_limit() {
    for next in [AgentState::Running, AgentState::Idle, AgentState::Input] {
        let mut store = AgentStore::new();
        let id = store.create("/tmp/a", CreateOptions::default());
        store.set_session_limit(id, Some(limit()));
        store.set_state(id, AgentState::Error);

        store.set_state(id, next);
        store.set_state(id, AgentState::Error);

        assert_eq!(store.agent(id).unwrap().active_session_limit(),
                   None,
                   "an Error after {next:?} is not the old limit");
    }
}

#[test]
fn a_new_limit_moves_the_revision_and_a_repeat_does_not() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let start = store.revision();

    store.set_session_limit(id, Some(limit()));
    let after = store.revision();
    assert_ne!(after, start);

    store.set_session_limit(id, Some(limit()));
    assert_eq!(store.revision(), after);
}

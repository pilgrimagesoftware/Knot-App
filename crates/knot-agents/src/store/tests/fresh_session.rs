//! `deactivate_fresh` and the guarded session-id write
//! (`agent-lifecycle`: "Deactivate with New Session", "A session id from
//! before a new session is not kept").

use super::super::*;

/// A running agent with every session field a resume would read.
fn running_agent_with_a_session(store: &mut AgentStore) -> Uuid {
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_activated(id, true);
    store.set_session_id(id, "s1".to_string());
    store.set_acp_session_id(id, "a1".to_string());
    store.request_fresh_context(id);
    let agent = store.agent_mut(id).unwrap();
    agent.resume_session_id = Some("s1".to_string());
    agent.fork_session = true;
    id
}

#[test]
fn deactivate_fresh_forgets_the_conversation_and_stops_the_agent() {
    let mut store = AgentStore::new();
    let id = running_agent_with_a_session(&mut store);
    let token = store.agent(id).unwrap().restart_token;

    assert_eq!(store.deactivate_fresh(id), vec![id]);

    let agent = store.agent(id).unwrap();
    assert!(!agent.activated, "the agent is not started again");
    assert_eq!(agent.session_to_load(), None);
    assert!(agent.session_id.is_none() && agent.acp_session_id.is_none());
    assert!(agent.resume_session_id.is_none());
    assert!(!agent.fork_session);
    assert!(!agent.fresh_context_requested, "aligned with restart");
    assert_ne!(agent.restart_token, token);
}

#[test]
fn deactivate_fresh_cascades_to_companions() {
    let mut store = AgentStore::new();
    let owner = running_agent_with_a_session(&mut store);
    let companion = store.create_shell_companion(owner).unwrap();
    store.set_activated(companion, true);

    let deactivated = store.deactivate_fresh(owner);

    assert_eq!(deactivated, vec![companion, owner]);
    assert!(!store.agent(companion).unwrap().activated);
}

#[test]
fn deactivate_fresh_of_an_unknown_agent_does_nothing() {
    let mut store = AgentStore::new();
    assert!(store.deactivate_fresh(Uuid::new_v4()).is_empty());
}

#[test]
fn a_session_id_under_the_current_token_is_stored() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let token = store.agent(id).unwrap().restart_token;

    assert!(store.set_acp_session_id_if_current(id, token, "a2".to_string()));
    assert_eq!(store.agent(id).unwrap().acp_session_id.as_deref(),
               Some("a2"));
}

#[test]
fn a_late_session_id_after_deactivate_fresh_is_dropped() {
    let mut store = AgentStore::new();
    let id = running_agent_with_a_session(&mut store);
    let token = store.agent(id).unwrap().restart_token;
    store.deactivate_fresh(id);

    assert!(!store.set_acp_session_id_if_current(id, token, "a2".to_string()));
    assert_eq!(store.agent(id).unwrap().session_to_load(), None);
}

#[test]
fn a_late_session_id_after_a_new_conversation_restart_is_dropped() {
    let mut store = AgentStore::new();
    let id = running_agent_with_a_session(&mut store);
    let token = store.agent(id).unwrap().restart_token;
    store.restart(id).unwrap();

    assert!(!store.set_acp_session_id_if_current(id, token, "a2".to_string()));
    assert!(store.agent(id).unwrap().acp_session_id.is_none());
}

#[test]
fn a_late_session_id_after_a_plain_deactivate_is_kept() {
    let mut store = AgentStore::new();
    let id = running_agent_with_a_session(&mut store);
    let token = store.agent(id).unwrap().restart_token;
    store.deactivate(id);

    assert!(store.set_acp_session_id_if_current(id, token, "a2".to_string()));
    assert_eq!(store.agent(id).unwrap().acp_session_id.as_deref(),
               Some("a2"));
}

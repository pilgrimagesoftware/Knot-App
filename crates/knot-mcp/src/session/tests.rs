//! The session table's contract, including who may bind and evict (D5).
use super::*;
use crate::auth::Principal;

#[test]
fn one_session_per_agent() {
    let manager = McpSessionManager::new();
    let agent_id = Uuid::new_v4();

    let first = manager.create_session(Principal::Agent(agent_id));
    let second = manager.create_session(Principal::Agent(agent_id));

    assert!(manager.session(&first.id).is_none());
    assert!(manager.session(&second.id).is_some());
    assert_eq!(manager.session_for_agent(agent_id).unwrap().id, second.id);
}

#[test]
fn binding_a_session_names_its_agent() {
    let manager = McpSessionManager::new();
    let agent = Uuid::new_v4();
    let session = manager.create_session(Principal::External);

    manager.bind(&session.id, agent);

    assert_eq!(manager.session_for_agent(agent).unwrap().id, session.id);
    assert!(!manager.held_elsewhere(agent, Some(&session.id)));
    assert!(manager.held_elsewhere(agent, None));
    assert!(manager.held_elsewhere(agent, Some("another")));
}

#[test]
fn rebinding_a_session_releases_its_old_agent() {
    let manager = McpSessionManager::new();
    let (old, new) = (Uuid::new_v4(), Uuid::new_v4());
    let session = manager.create_session(Principal::Agent(old));

    manager.bind(&session.id, new);

    assert!(manager.session_for_agent(old).is_none());
    assert!(!manager.held_elsewhere(old, None));
    assert_eq!(manager.session(&session.id).unwrap().agent_id, new);
}

#[test]
fn a_removed_session_holds_nothing() {
    let manager = McpSessionManager::new();
    let agent = Uuid::new_v4();
    let session = manager.create_session(Principal::Agent(agent));
    manager.remove(&session.id);
    assert!(!manager.held_elsewhere(agent, None));
}

#[test]
fn nil_agent_sessions_do_not_evict_each_other() {
    let manager = McpSessionManager::new();

    let first = manager.create_session(Principal::External);
    let second = manager.create_session(Principal::External);

    assert!(manager.session(&first.id).is_some());
    assert!(manager.session(&second.id).is_some());
}

#[test]
fn the_live_count_tracks_creation_replacement_and_cleanup() {
    let manager = McpSessionManager::new();
    assert_eq!(manager.len(), 0);
    assert!(manager.is_empty());

    let agent = Uuid::new_v4();
    manager.create_session(Principal::Agent(agent));
    manager.create_session(Principal::Agent(Uuid::new_v4()));
    assert_eq!(manager.len(), 2);
    assert!(!manager.is_empty());

    // Replacing an agent's session leaves the count alone: the old one
    // is removed as the new one lands, so a reconnecting agent must not
    // read as a second connection.
    manager.create_session(Principal::Agent(agent));
    assert_eq!(manager.len(), 2);

    manager.remove_for_agent(agent);
    assert_eq!(manager.len(), 1);

    std::thread::sleep(Duration::from_millis(20));
    manager.cleanup_stale(Duration::from_millis(10));
    assert_eq!(manager.len(), 0, "a reclaimed session stops being counted");
}

#[test]
fn cleanup_stale_removes_only_expired_sessions() {
    let manager = McpSessionManager::new();
    let stale_agent = Uuid::new_v4();
    let fresh_agent = Uuid::new_v4();

    let stale = manager.create_session(Principal::Agent(stale_agent));
    std::thread::sleep(Duration::from_millis(30));
    let fresh = manager.create_session(Principal::Agent(fresh_agent));

    manager.cleanup_stale(Duration::from_millis(15));

    assert!(manager.session(&stale.id).is_none());
    assert!(manager.session(&fresh.id).is_some());
}

#[test]
fn a_restarted_agent_takes_over_with_its_new_credential() {
    let manager = McpSessionManager::new();
    let agent = Uuid::new_v4();
    let before = manager.create_session(Principal::Agent(agent));

    let after = manager.create_session(Principal::Agent(agent));

    assert!(manager.session(&before.id).is_none());
    assert_eq!(manager.session_for_agent(agent).unwrap().id, after.id);
    assert!(manager.held_by_credential(agent));
}

#[test]
fn the_client_token_cannot_displace_an_agents_own_session() {
    let manager = McpSessionManager::new();
    let agent = Uuid::new_v4();
    let held = manager.create_session(Principal::Agent(agent));
    let external = manager.create_session(Principal::External);

    manager.bind(&external.id, agent);

    assert_eq!(manager.session_for_agent(agent).unwrap().id, held.id);
    assert!(manager.session(&external.id).unwrap().agent_id.is_nil());
}

#[test]
fn the_client_token_binds_an_agent_with_no_live_session() {
    let manager = McpSessionManager::new();
    let agent = Uuid::new_v4();
    let external = manager.create_session(Principal::External);

    manager.bind(&external.id, agent);

    assert_eq!(manager.session_for_agent(agent).unwrap().id, external.id);
    assert!(!manager.held_by_credential(agent),
            "bound by registering, not by the agent's credential");
}

#[test]
fn a_session_answers_only_the_principal_that_opened_it() {
    let manager = McpSessionManager::new();
    let (agent, other) = (Uuid::new_v4(), Uuid::new_v4());
    let session = manager.create_session(Principal::Agent(agent));

    assert!(manager.session_for(&session.id, Principal::Agent(agent))
                   .is_some());
    assert!(manager.session_for(&session.id, Principal::Agent(other))
                   .is_none());
    assert!(manager.session_for(&session.id, Principal::External)
                   .is_none());
    assert!(manager.session_for(&session.id, Principal::AgentHooks(agent))
                   .is_none());
}

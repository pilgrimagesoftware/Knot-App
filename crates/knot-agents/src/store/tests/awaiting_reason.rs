//! `Agent::awaiting_reason`: why an agent showing "Awaiting input" is
//! waiting, kept exactly as long as the wait (#344).

use super::super::*;

fn reason(store: &AgentStore, id: Uuid) -> Option<String> {
    store.agent(id)
         .and_then(|agent| agent.input_reason())
         .map(str::to_owned)
}

#[test]
fn a_reason_is_shown_while_the_agent_waits() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());

    store.set_state(id, AgentState::Input);
    store.set_awaiting_reason(id, Some("permission requested for Bash".to_string()));

    assert_eq!(reason(&store, id).as_deref(),
               Some("permission requested for Bash"));
}

/// The two arrive from different sinks, so the reason may land first.
#[test]
fn a_reason_given_before_the_state_moves_is_kept() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());

    store.set_awaiting_reason(id, Some("asks".to_string()));
    assert_eq!(reason(&store, id), None, "not shown until the agent waits");
    store.set_state(id, AgentState::Input);

    assert_eq!(reason(&store, id).as_deref(), Some("asks"));
}

/// Whatever ends the wait - an answer, Stop, a new turn - ends its reason.
#[test]
fn leaving_input_by_any_state_clears_the_reason() {
    for next in [AgentState::Running, AgentState::Idle, AgentState::Error] {
        let mut store = AgentStore::new();
        let id = store.create("/tmp/a", CreateOptions::default());
        store.set_state(id, AgentState::Input);
        store.set_awaiting_reason(id, Some("asks".to_string()));

        store.set_state(id, next);
        store.set_state(id, AgentState::Input);

        assert_eq!(reason(&store, id),
                   None,
                   "a later wait after {next:?} starts unexplained");
    }
}

#[test]
fn a_wait_re_reported_without_a_reason_keeps_the_one_it_had() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_state(id, AgentState::Input);
    store.set_awaiting_reason(id, Some("asks".to_string()));

    store.set_awaiting_reason(id, None);
    store.set_awaiting_reason(id, Some("  ".to_string()));

    assert_eq!(reason(&store, id).as_deref(), Some("asks"));
}

/// Written from tracker and MCP threads with no context to notify from, so
/// it rides the store revision to the screen (#17).
#[test]
fn a_new_reason_moves_the_revision_and_a_repeat_does_not() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_state(id, AgentState::Input);
    let start = store.revision();

    store.set_awaiting_reason(id, Some("asks".to_string()));
    let after = store.revision();
    assert_ne!(after, start);

    store.set_awaiting_reason(id, Some("asks".to_string()));
    assert_eq!(store.revision(), after);
}

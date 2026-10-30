//! `AgentStore::revision`: how a window notices a change it did not make
//! itself, such as `set-status` text written from the MCP server's thread
//! (#17).

use super::super::*;

#[test]
fn new_status_text_moves_the_revision_and_a_repeat_does_not() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let start = store.revision();

    store.set_status_text(id, "planning".to_string());
    let after = store.revision();
    assert_ne!(after, start, "new text has to reach the screen");

    store.set_status_text(id, "planning".to_string());
    assert_eq!(store.revision(),
               after,
               "the same text again costs no repaint");
}

#[test]
fn a_state_change_moves_the_revision_and_a_repeat_does_not() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let start = store.revision();

    store.set_state(id, AgentState::Running);
    let after = store.revision();
    assert_ne!(after, start);

    store.set_state(id, AgentState::Running);
    assert_eq!(store.revision(), after);
}

#[test]
fn a_write_to_an_agent_that_is_gone_moves_nothing() {
    let mut store = AgentStore::new();
    let start = store.revision();

    store.set_status_text(Uuid::new_v4(), "planning".to_string());

    assert_eq!(store.revision(), start);
}

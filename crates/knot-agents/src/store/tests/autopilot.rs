//! Turning Autopilot off for one agent (#323).

use super::super::*;

#[test]
fn autopilot_applies_while_on_in_settings_and_not_turned_off_for_the_agent() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let agent = |store: &AgentStore| store.agent(id).unwrap().clone();

    assert!(agent(&store).autopilot_applies(true));
    assert!(!agent(&store).autopilot_applies(false),
            "off in Settings is off everywhere");

    store.set_autopilot_off(id, true);
    assert!(!agent(&store).autopilot_applies(true));

    store.set_autopilot_off(id, false);
    assert!(agent(&store).autopilot_applies(true));
}

#[test]
fn a_shell_agent_is_never_read_by_autopilot() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a",
                          CreateOptions { agent_type:
                                              Some(knot_core::agent_type::SHELL.to_string()),
                                          ..Default::default() });

    assert!(!store.agent(id).unwrap().autopilot_applies(true));
}

/// The setting is the user's, so it is kept across a relaunch; a duplicate
/// is a new session and follows Settings again.
#[test]
fn autopilot_off_survives_a_reload_but_not_a_duplicate() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_autopilot_off(id, true);

    let reloaded = AgentStore::from_saved(&store.saved_agents(false), Vec::new());
    assert!(reloaded.agent(id).unwrap().autopilot_off);

    let copy = store.duplicate(id).unwrap();
    assert!(!store.agent(copy).unwrap().autopilot_off);
}

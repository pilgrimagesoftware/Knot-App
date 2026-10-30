//! The sweep that keeps an activated agent running (#316, #320).

use std::collections::BTreeSet;

use uuid::Uuid;

use super::{AgentRun, agents_to_start};

fn agent(activated: bool, has_session: bool) -> AgentRun {
    AgentRun { id: Uuid::new_v4(),
               activated,
               has_session }
}

/// The reported bug: an agent lit in the sidebar with nothing running.
#[test]
fn an_activated_agent_without_a_session_is_started() {
    let lit = agent(true, false);
    let mut requested = BTreeSet::new();

    assert_eq!(agents_to_start(&[lit], &mut requested), vec![lit.id]);
}

#[test]
fn a_running_or_stopped_agent_is_left_alone() {
    let running = agent(true, true);
    let stopped = agent(false, false);
    let mut requested = BTreeSet::new();

    assert!(agents_to_start(&[running, stopped], &mut requested).is_empty(),
            "a running agent needs nothing, and a deactivated or passive one must not start");
}

/// A session that cannot start is not retried at the poll's rate.
#[test]
fn an_agent_is_started_once_per_activation() {
    let lit = agent(true, false);
    let mut requested = BTreeSet::new();

    agents_to_start(&[lit], &mut requested);
    assert!(agents_to_start(&[lit], &mut requested).is_empty(),
            "the second tick must not start it again");
}

/// Deactivating forgets the claim, so the next activation starts it again.
#[test]
fn a_reactivated_agent_is_started_again() {
    let lit = agent(true, false);
    let mut requested = BTreeSet::new();
    agents_to_start(&[lit], &mut requested);

    agents_to_start(&[AgentRun { activated: false,
                                 ..lit }],
                    &mut requested);

    assert_eq!(agents_to_start(&[lit], &mut requested), vec![lit.id]);
}

#[test]
fn an_agent_that_is_gone_is_forgotten() {
    let lit = agent(true, false);
    let mut requested = BTreeSet::new();
    agents_to_start(&[lit], &mut requested);

    agents_to_start(&[], &mut requested);

    assert!(requested.is_empty());
}

/// Against a real window: an agent created `Active` into a workspace that is
/// already open - what MCP `create-agent` does - is started by the sweep, not
/// left lit and idle until its row is clicked. A passive one is left alone.
#[gpui_kit::test]
fn an_agent_created_active_into_an_open_workspace_is_started(cx: &mut gpui_kit::TestAppContext) {
    let mut fixture = crate::workspace_window::shortcuts_tests::window_with_agents(1, cx);

    fixture.view.update(&mut fixture.window, |view, cx| {
                    let (active, passive) = {
                        let mut store = view.store.lock();
                        let create = |store: &mut knot_agents::AgentStore, mode| {
                            store.create("~/created",
                                         knot_agents::CreateOptions {
                                             agent_type: Some("keybindings-test-inert".into()),
                                             workspace_id: Some(view.workspace_id),
                                             activation_mode: mode,
                                             ..Default::default()
                                         })
                        };
                        (create(&mut store, knot_core::ActivationMode::Active),
                         create(&mut store, knot_core::ActivationMode::Passive))
                    };

                    assert!(view.start_activated_agents(cx), "the lit agent is started");
                    assert!(view.start_requested.contains(&active));
                    assert!(!view.start_requested.contains(&passive),
                            "a passive agent waits to be selected or messaged");
                    assert!(!view.start_activated_agents(cx),
                            "and it is started once, not on every tick");
                });
}

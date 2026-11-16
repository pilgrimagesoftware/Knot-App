//! The summary's counting, and its catalog keys.

use knot_agents::AgentState;
use uuid::Uuid;

use super::*;

fn agent(state: AgentState, has_session: bool, is_shell: bool) -> DashboardAgent {
    DashboardAgent { id: Uuid::new_v4(),
                     avatar: String::new(),
                     name: String::new(),
                     folder_name: String::new(),
                     state,
                     is_shell,
                     header_title: String::new(),
                     input_reason: None,
                     session_limit: None,
                     git_stats: None,
                     has_session }
}

#[test]
fn counts_come_in_order_without_zeros() {
    let agents = [agent(AgentState::Idle, true, false),
                  agent(AgentState::Running, true, false),
                  agent(AgentState::Idle, true, false),
                  agent(AgentState::Input, true, false)];
    assert_eq!(status_counts(&agents),
               vec![(WorkingIndicatorState::AwaitingInput, 1),
                    (WorkingIndicatorState::Working, 1),
                    (WorkingIndicatorState::Idle, 2)]);
}

/// An agent with no session counts as not running, whatever state it last
/// stored - the case a closed window left behind.
#[test]
fn an_agent_without_a_session_is_not_running() {
    let agents = [agent(AgentState::Running, false, false)];
    assert_eq!(status_counts(&agents),
               vec![(WorkingIndicatorState::Off, 1)]);
}

#[test]
fn shell_agents_are_not_counted() {
    let agents = [agent(AgentState::Idle, true, true)];
    assert!(status_counts(&agents).is_empty());
}

#[test]
fn every_label_key_resolves_and_keeps_its_count() {
    for state in ORDER {
        let (one, many) = label_keys(state);
        assert_ne!(knot_core::l10n::t(one), one);
        assert_ne!(knot_core::l10n::t(many), many);
        assert!(knot_core::l10n::pluralize(3, one, many).starts_with('3'));
    }
}

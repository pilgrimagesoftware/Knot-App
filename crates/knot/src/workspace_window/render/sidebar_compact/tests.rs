//! Unit tests for [`super::compact_tooltip`] (#253): a compact row's tooltip
//! carries the agent's status as well as its name.

use knot_agents::AgentState;

use super::{CompactTooltip, compact_tooltip};

fn tooltip(state: AgentState, status: &str) -> CompactTooltip {
    CompactTooltip { name: "Builder".into(),
                     state,
                     idle_since: None,
                     input_reason: None,
                     session_limit: None,
                     is_shell: false,
                     status: status.into(),
                     scheduling: Vec::new() }
}

#[test]
fn the_name_then_the_state_then_the_status_line() {
    let text = compact_tooltip(&tooltip(AgentState::Running, "Running tests"));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines,
               ["Builder",
                crate::app_state::state_label(AgentState::Running),
                "Running tests"]);
}

/// The state reads as the status dot's tooltip reads, waiting reason
/// included, so the two cannot disagree.
#[test]
fn the_state_is_worded_as_the_dot_words_it() {
    let mut waiting = tooltip(AgentState::Input, "");
    waiting.input_reason = Some("Allow `rm -rf build`?".into());
    let expected = crate::app_state::state_tooltip(AgentState::Input,
                                                   None,
                                                   Some("Allow `rm -rf build`?"),
                                                   None);
    assert_eq!(compact_tooltip(&waiting), format!("Builder\n{expected}"));
}

#[test]
fn an_empty_status_line_is_left_out() {
    assert_eq!(compact_tooltip(&tooltip(AgentState::Idle, "  ")).lines()
                                                                .count(),
               2);
}

/// A shell agent has no state - it has no dot - so its tooltip names it and
/// what it is running.
#[test]
fn a_shell_agent_shows_no_state() {
    let mut shell = tooltip(AgentState::Idle, "zsh");
    shell.is_shell = true;
    assert_eq!(compact_tooltip(&shell), "Builder\nzsh");
}

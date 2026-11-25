//! Which lines carry a Shift alternate (#475): the agent row's folded lines,
//! the menu bar's list, and the sidebar's Restart All and Deactivate All
//! (`agent-list-ui`, `app-menu`). The open-menu behaviour is in
//! `workspace_window::menu_alternates_tests`.

use crate::agent_menu_entries::AgentMenuEntry;
use crate::agent_menu_entries::AgentMenuFacts;
use crate::agent_menu_entries::AgentMenuLine;
use crate::agent_menu_entries::agent_context_menu_entries;
use crate::agent_menu_entries::agent_context_menu_lines;
use crate::app_state::AgentListBackgroundEntry;

/// A running owner that is not a shell.
fn running_owner(restores_conversations: bool) -> AgentMenuFacts {
    AgentMenuFacts { is_running: true,
                     ..Default::default() }.with_restored_conversations(restores_conversations)
}

fn lines_with_alternates(facts: AgentMenuFacts) -> Vec<(AgentMenuEntry, AgentMenuEntry)> {
    agent_context_menu_lines(facts).into_iter()
                                   .filter_map(|AgentMenuLine { entry, alternate }| {
                                       alternate.map(|alternate| (entry, alternate))
                                   })
                                   .collect()
}

#[test]
fn restart_and_deactivate_carry_their_alternates() {
    assert_eq!(lines_with_alternates(running_owner(true)),
               [(AgentMenuEntry::Deactivate, AgentMenuEntry::DeactivateWithNewSession),
                (AgentMenuEntry::RestartAgent, AgentMenuEntry::RestartWithNewConversation)]);
}

#[test]
fn an_alternate_is_never_a_line_of_its_own() {
    let entries = agent_context_menu_lines(running_owner(true)).into_iter()
                                                               .map(|line| line.entry)
                                                               .collect::<Vec<_>>();
    assert!(!entries.contains(&AgentMenuEntry::DeactivateWithNewSession));
    assert!(!entries.contains(&AgentMenuEntry::RestartWithNewConversation));
}

#[test]
fn no_alternates_without_restored_conversations() {
    assert!(lines_with_alternates(running_owner(false)).is_empty());
    let entries = agent_context_menu_entries(running_owner(false));
    assert!(!entries.contains(&AgentMenuEntry::DeactivateWithNewSession));
    assert!(!entries.contains(&AgentMenuEntry::RestartWithNewConversation));
}

#[test]
fn no_alternates_for_a_shell_or_a_companion() {
    let shell = AgentMenuFacts { is_shell: true,
                                 ..running_owner(true) };
    assert!(lines_with_alternates(shell).is_empty());
    let companion = AgentMenuFacts { is_companion: true,
                                     is_shell: true,
                                     ..running_owner(true) };
    assert!(lines_with_alternates(companion).is_empty());
}

#[test]
fn deactivate_with_new_session_needs_a_running_agent() {
    let stopped = AgentMenuFacts { is_running: false,
                                   ..running_owner(true) };
    assert_eq!(lines_with_alternates(stopped),
               [(AgentMenuEntry::RestartAgent, AgentMenuEntry::RestartWithNewConversation)]);
}

/// The menu bar lists each alternate as its own item, directly below its
/// primary, until native alternates fold them.
#[test]
fn the_menu_bar_lists_each_alternate_below_its_primary() {
    let entries = agent_context_menu_entries(AgentMenuFacts::EVERY_ITEM);
    for (primary, alternate) in
        [(AgentMenuEntry::Deactivate, AgentMenuEntry::DeactivateWithNewSession),
         (AgentMenuEntry::RestartAgent, AgentMenuEntry::RestartWithNewConversation)]
    {
        let at = entries.iter().position(|entry| *entry == primary);
        assert_eq!(at.map(|at| entries.get(at + 1)),
                   Some(Some(&alternate)),
                   "{entries:?}");
    }
}

#[test]
fn the_sidebar_offers_restart_all_and_deactivate_all_alternates() {
    assert_eq!(AgentListBackgroundEntry::RestartAll.shift_alternate(true),
               Some(AgentListBackgroundEntry::RestartAllNewConversations));
    assert_eq!(AgentListBackgroundEntry::DeactivateAll.shift_alternate(true),
               Some(AgentListBackgroundEntry::DeactivateAllNewSessions));
    for entry in [AgentListBackgroundEntry::RestartAll,
                  AgentListBackgroundEntry::DeactivateAll]
    {
        assert_eq!(entry.shift_alternate(false), None, "{entry:?}");
    }
    for entry in [AgentListBackgroundEntry::NewAgent,
                  AgentListBackgroundEntry::NewFromBench,
                  AgentListBackgroundEntry::CloseAll,
                  AgentListBackgroundEntry::Broadcast]
    {
        assert_eq!(entry.shift_alternate(true), None, "{entry:?}");
    }
}

#[test]
fn the_sidebar_alternate_labels_resolve() {
    for entry in [AgentListBackgroundEntry::RestartAllNewConversations,
                  AgentListBackgroundEntry::DeactivateAllNewSessions]
    {
        let label = entry.label().expect("a labelled entry");
        assert!(!label.starts_with("menu.sidebar."), "{entry:?}: {label}");
    }
}

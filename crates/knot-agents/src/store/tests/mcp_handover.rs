//! Handover terminals (#443): one per owner and server, recognisable after a
//! reload, and removable without touching ordinary companions.

use knot_core::ActivationMode;
use uuid::Uuid;

use crate::{
    AgentError, AgentStore, CreateOptions, McpHandoverRequest, McpHandoverTerminal, from_saved,
    to_saved,
};

fn owner(store: &mut AgentStore) -> Uuid {
    store.create("/repo",
                 CreateOptions { activation_mode: ActivationMode::Passive,
                                 ..Default::default() })
}

fn request(owner: Uuid, server: &str) -> McpHandoverRequest<'_> {
    McpHandoverRequest { owner,
                         server,
                         command: "claude mcp",
                         name: "MCP" }
}

fn companions(store: &AgentStore, owner: Uuid) -> usize {
    store.companions(owner).len()
}

#[test]
fn a_repeated_action_keeps_one_terminal() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);

    let first = store.open_mcp_handover(request(owner, "github"))
                     .expect("an owner can hand over");
    let second = store.open_mcp_handover(request(owner, "github"))
                      .expect("an owner can hand over");

    assert!(matches!(first, McpHandoverTerminal::Created(_)),
            "{first:?}");
    assert_eq!(second, McpHandoverTerminal::Reused(first.id()));
    assert_eq!(companions(&store, owner), 1);
}

#[test]
fn another_server_gets_its_own_terminal() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);

    let github = store.open_mcp_handover(request(owner, "github")).unwrap();
    let linear = store.open_mcp_handover(request(owner, "linear")).unwrap();

    assert_ne!(github.id(), linear.id());
    assert_eq!(companions(&store, owner), 2);
}

#[test]
fn the_terminal_is_a_marked_shell_companion_running_the_command() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);

    let id = store.open_mcp_handover(request(owner, "github"))
                  .unwrap()
                  .id();

    let terminal = store.agent(id).expect("the terminal");
    assert!(terminal.is_companion);
    assert_eq!(terminal.created_by, Some(owner));
    assert!(knot_core::agent_type::is_shell(&terminal.agent_type));
    assert_eq!(terminal.shell_command.as_deref(), Some("claude mcp"));
    assert_eq!(terminal.mcp_handover.as_deref(), Some("github"));
    assert_eq!(store.mcp_handover_owner(id), Some(owner));
}

#[test]
fn a_companion_cannot_own_a_handover() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);
    let shell = store.create_shell_companion(owner).unwrap();

    let refused = store.open_mcp_handover(request(shell, "github"));

    assert!(matches!(refused, Err(AgentError::CompanionCannotOwn(id)) if id == shell),
            "{refused:?}");
}

#[test]
fn an_ordinary_companion_is_not_a_handover() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);
    let shell = store.create_shell_companion(owner).unwrap();

    assert_eq!(store.mcp_handover_owner(shell), None);
    assert_eq!(store.mcp_handover_terminal(owner, "github"), None);
}

/// The marker is durable: after a save and reload the terminal is still known
/// for what it is, which is what lets the next run clean it up.
#[test]
fn the_marker_survives_a_reload() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);
    let id = store.open_mcp_handover(request(owner, "github"))
                  .unwrap()
                  .id();

    let saved = to_saved(store.agent(id).unwrap(), false);
    let json = serde_json::to_string(&saved).expect("serializes");
    let loaded: knot_core::SavedAgent = serde_json::from_str(&json).expect("deserializes");

    assert_eq!(from_saved(&loaded).mcp_handover.as_deref(), Some("github"));
}

/// Every other agent's record is unchanged on disk, and one written before the
/// field existed loads unmarked.
#[test]
fn an_unmarked_record_neither_writes_nor_needs_the_field() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);

    let json = serde_json::to_string(&to_saved(store.agent(owner).unwrap(), false)).unwrap();
    assert!(!json.contains("mcpHandover"), "{json}");

    let loaded: knot_core::SavedAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.mcp_handover, None);
}

/// An unstarted terminal does not outlive the run or window that opened it:
/// removal takes the handover terminals and leaves everything else, including
/// the owner's ordinary companions and the workspace's other members.
#[test]
fn removal_takes_only_handover_terminals() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);
    let shell = store.create_shell_companion(owner).unwrap();
    let terminal = store.open_mcp_handover(request(owner, "github"))
                        .unwrap()
                        .id();
    let ids: Vec<Uuid> = store.agents().iter().map(|agent| agent.id).collect();

    let removed: Vec<Uuid> = store.remove_mcp_handovers(&ids)
                                  .into_iter()
                                  .map(|agent| agent.id)
                                  .collect();

    assert_eq!(removed, [terminal]);
    assert!(store.agent(terminal).is_none());
    assert!(store.agent(owner).is_some() && store.agent(shell).is_some());
    assert!(store.workspaces()
                 .iter()
                 .all(|workspace| !workspace.agent_ids.contains(&terminal)),
            "the workspace must not keep naming a removed terminal");
}

#[test]
fn removal_is_limited_to_the_ids_given() {
    let mut store = AgentStore::default();
    let owner = owner(&mut store);
    let terminal = store.open_mcp_handover(request(owner, "github"))
                        .unwrap()
                        .id();

    assert!(store.remove_mcp_handovers(&[owner]).is_empty());
    assert!(store.agent(terminal).is_some());
}

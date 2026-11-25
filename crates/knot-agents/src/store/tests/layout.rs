//! The store applying the layout rules (`workspace-layouts`): removal, a move,
//! a new companion, and a saved layout loading. The rules themselves are
//! tested in `layout/tests.rs`; this covers that every path reaches them.

use std::collections::BTreeMap;

use knot_core::{LayoutMode, WorkspaceUiState};

use super::super::*;
use crate::CompanionOrigin;

/// Agents `a` to `d` in the store's first workspace, shown in a grid with
/// pane 0 focused.
fn grid() -> (AgentStore, Uuid, [Uuid; 4]) {
    let mut store = AgentStore::new();
    let ids = ["a", "b", "c", "d"].map(|folder| {
                                      store.create(format!("/tmp/{folder}"),
                                                   CreateOptions::default())
                                  });
    let workspace = store.workspaces()[0].id;
    store.set_workspace_layout(workspace, LayoutMode::GridFourPane, ids.to_vec());
    (store, workspace, ids)
}

fn workspace_named(name: &str) -> knot_core::Workspace {
    knot_core::Workspace { id:        Uuid::new_v4(),
                           name:      name.to_string(),
                           color_hex: "#000000".to_string(),
                           agent_ids: Vec::new(), }
}

#[test]
fn removing_a_panes_agent_collapses_the_mode() {
    let (mut store, workspace, [a, b, c, d]) = grid();
    store.set_workspace_focused_pane(workspace, 3);
    store.remove(c);
    let ui = store.workspace_ui(workspace);
    assert_eq!(ui.layout_mode, LayoutMode::ThreePane);
    assert_eq!(ui.active_agent_ids, vec![a, b, d]);
    assert_eq!(ui.focused_pane_index, 2, "clamped to the panes that remain");
    store.remove(d);
    store.remove(b);
    let ui = store.workspace_ui(workspace);
    assert_eq!(ui.layout_mode, LayoutMode::Single);
    assert_eq!(ui.active_agent_ids, vec![a]);
}

#[test]
fn removing_the_only_shown_agent_shows_the_first_remaining() {
    let mut store = AgentStore::new();
    let a = store.create("/tmp/a", CreateOptions::default());
    let b = store.create("/tmp/b", CreateOptions::default());
    let workspace = store.workspaces()[0].id;
    assert_eq!(store.workspace_ui(workspace).active_agent_ids, vec![a]);
    store.remove(a);
    assert_eq!(store.workspace_ui(workspace).active_agent_ids, vec![b]);
}

#[test]
fn removing_an_owner_collapses_the_split_it_shared_with_its_companion() {
    let mut store = AgentStore::new();
    let a = store.create("/tmp/a", CreateOptions::default());
    let workspace = store.workspaces()[0].id;
    let shell = store.create_shell_companion(a).unwrap();
    assert_eq!(store.workspace_ui(workspace).active_agent_ids,
               vec![a, shell]);
    store.remove(shell);
    let ui = store.workspace_ui(workspace);
    assert_eq!((ui.layout_mode, ui.active_agent_ids),
               (LayoutMode::Single, vec![a]));
}

#[test]
fn a_move_collapses_the_old_layout_and_leaves_the_target_alone() {
    let (mut store, source, [a, b, c, d]) = grid();
    let target = workspace_named("target");
    let target_id = target.id;
    store.add_workspace(target);
    let other = store.create("/tmp/e",
                             CreateOptions { workspace_id: Some(target_id),
                                             ..Default::default() });
    let before = store.workspace_ui(target_id);
    assert_eq!(before.active_agent_ids, vec![other]);

    assert!(store.move_to_workspace(b, target_id));

    let ui = store.workspace_ui(source);
    assert_eq!(ui.layout_mode, LayoutMode::ThreePane);
    assert_eq!(ui.active_agent_ids, vec![a, c, d]);
    assert_eq!(store.workspace_ui(target_id),
               before,
               "the moved agent is in the target's roster, not its panes");
}

#[test]
fn a_move_shows_the_agent_in_a_target_that_showed_nothing() {
    let (mut store, _source, [a, ..]) = grid();
    let target = workspace_named("target");
    let target_id = target.id;
    store.add_workspace(target);
    store.move_to_workspace(a, target_id);
    assert_eq!(store.workspace_ui(target_id).active_agent_ids, vec![a]);
}

#[test]
fn a_companion_opens_beside_its_owner() {
    let mut store = AgentStore::new();
    let a = store.create("/tmp/a", CreateOptions::default());
    let workspace = store.workspaces()[0].id;
    let shell = store.create_shell_companion(a).unwrap();
    let ui = store.workspace_ui(workspace);
    assert_eq!(ui.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(ui.active_agent_ids, vec![a, shell]);
    assert_eq!(ui.focused_pane_index, 1, "the companion is focused");

    let second = store.create_shell_companion(a).unwrap();
    let ui = store.workspace_ui(workspace);
    assert_eq!(ui.layout_mode, LayoutMode::ThreePane);
    assert_eq!(ui.active_agent_ids, vec![a, shell, second]);
}

#[test]
fn a_user_companion_of_an_owner_that_is_not_shown_replaces_the_layout() {
    let (mut store, workspace, [a, b, ..]) = grid();
    store.set_workspace_layout(workspace, LayoutMode::SplitVertical, vec![b, a]);
    let c = store.workspaces()[0].agent_ids[2];
    let shell = store.create_shell_companion(c).unwrap();
    let ui = store.workspace_ui(workspace);
    assert_eq!(ui.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(ui.active_agent_ids, vec![c, shell]);
    assert_eq!(ui.focused_pane_index, 1);
}

#[test]
fn a_companion_created_another_way_leaves_the_layout_alone_unless_the_owner_is_shown() {
    let (mut store, workspace, [a, b, c, _d]) = grid();
    store.set_workspace_layout(workspace, LayoutMode::SplitVertical, vec![b, c]);
    let before = store.workspace_ui(workspace);
    let companion = |store: &mut AgentStore, owner, origin| {
        store.create("/tmp/x",
                     CreateOptions { created_by: Some(owner),
                                     is_companion: true,
                                     insert_after: Some(owner),
                                     companion_origin: origin,
                                     ..Default::default() })
    };
    // Over MCP, with the owner elsewhere.
    companion(&mut store, a, Some(CompanionOrigin::Other));
    assert_eq!(store.workspace_ui(workspace), before);
    // No origin recorded, the handover terminal's case, even beside a shown
    // owner.
    companion(&mut store, b, None);
    assert_eq!(store.workspace_ui(workspace), before);
    // Over MCP beside an owner that is shown still opens beside it.
    let beside = companion(&mut store, b, Some(CompanionOrigin::Other));
    assert_eq!(store.workspace_ui(workspace).active_agent_ids,
               vec![b, c, beside]);
}

#[test]
fn a_grid_with_a_missing_agent_loads_as_three_pane() {
    let (mut store, workspace, [a, b, _c, d]) = grid();
    // The saved grid names an agent this store does not hold in pane 2.
    let ui = WorkspaceUiState { layout_mode: LayoutMode::GridFourPane,
                                active_agent_ids: vec![a, b, Uuid::new_v4(), d],
                                ..Default::default() };
    store.set_workspace_ui(BTreeMap::from([(workspace, ui)]));
    let ui = store.workspace_ui(workspace);
    assert_eq!(ui.layout_mode, LayoutMode::ThreePane);
    assert_eq!(ui.active_agent_ids, vec![a, b, d]);
}

#[test]
fn a_stale_focused_index_is_clamped_on_load() {
    let (mut store, workspace, [a, b, ..]) = grid();
    let ui = WorkspaceUiState { layout_mode: LayoutMode::SplitVertical,
                                active_agent_ids: vec![a, b],
                                focused_pane_index: 9,
                                ..Default::default() };
    store.set_workspace_ui(BTreeMap::from([(workspace, ui)]));
    let ui = store.workspace_ui(workspace);
    assert_eq!(ui.active_agent_ids, vec![a, b]);
    assert_eq!(ui.focused_pane_index, 1);
}

#[test]
fn a_ui_entry_for_a_workspace_the_store_does_not_hold_is_left_as_it_is() {
    let mut store = AgentStore::new();
    let ghost = Uuid::new_v4();
    let ui = WorkspaceUiState { layout_mode: LayoutMode::SplitVertical,
                                active_agent_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
                                ..Default::default() };
    store.set_workspace_ui(BTreeMap::from([(ghost, ui.clone())]));
    assert_eq!(store.workspace_ui(ghost), ui);
}

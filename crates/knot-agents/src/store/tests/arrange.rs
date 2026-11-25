//! The store's layout actions: they build the agent order and the companion
//! lookup from the workspace, apply the pure rules (`layout/tests.rs` covers
//! those) and write the result.

use knot_core::LayoutMode;

use super::super::*;

/// A workspace holding `a`, `b`, `c` in sidebar order, shown alone as `a`.
fn trio() -> (AgentStore, Uuid, [Uuid; 3]) {
    let mut store = AgentStore::new();
    let ids = ["a", "b", "c"].map(|folder| {
                                 store.create(format!("/tmp/{folder}"), CreateOptions::default())
                             });
    let workspace = store.workspaces()[0].id;
    (store, workspace, ids)
}

#[test]
fn selecting_replaces_the_focused_pane_and_reports_what_was_placed() {
    let (mut store, workspace, [a, b, c]) = trio();
    store.set_workspace_layout(workspace, LayoutMode::SplitVertical, vec![a, b]);
    store.set_workspace_focused_pane(workspace, 1);

    let change = store.select_in_layout(workspace, c, false);

    assert_eq!(change.after.active_agent_ids, vec![a, c]);
    assert_eq!(change.focused(), Some(c));
    assert_eq!(change.placed(), vec![c], "c was in no pane before");
    assert!(change.changed());
    assert_eq!(store.workspace_ui(workspace),
               change.after,
               "and it is written");
}

#[test]
fn selecting_an_agent_already_shown_only_moves_focus() {
    let (mut store, workspace, [a, b, _]) = trio();
    store.set_workspace_layout(workspace, LayoutMode::SplitVertical, vec![a, b]);

    let change = store.select_in_layout(workspace, b, false);

    assert_eq!(change.after.active_agent_ids, vec![a, b]);
    assert_eq!(change.focused(), Some(b));
    assert!(change.placed().is_empty());
}

#[test]
fn the_companion_lookup_is_the_workspaces_in_sidebar_order() {
    let (mut store, workspace, [a, b, _]) = trio();
    let first = store.create_shell_companion(a).unwrap();
    let second = store.create_shell_companion(a).unwrap();
    store.set_workspace_layout(workspace, LayoutMode::Single, vec![b]);

    // Owner a has companions, pane 0 is focused, no modifier: R1.
    let change = store.select_in_layout(workspace, a, false);
    assert_eq!(change.after.layout_mode, LayoutMode::ThreePane);
    assert_eq!(change.after.active_agent_ids.len(), 3);
    assert_eq!(change.after.active_agent_ids[0], a);
    let mut companions = store.workspace_companions(workspace, a);
    assert_eq!(companions.len(), 2);
    companions.sort();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(companions, expected);
    assert_eq!(&change.after.active_agent_ids[1..],
               store.workspace_companions(workspace, a).as_slice(),
               "the panes follow the sidebar order");
}

#[test]
fn command_skips_the_companion_layout() {
    let (mut store, workspace, [a, b, _]) = trio();
    store.create_shell_companion(a).unwrap();
    store.set_workspace_layout(workspace, LayoutMode::Single, vec![b]);

    let change = store.select_in_layout(workspace, a, true);

    assert_eq!(change.after.layout_mode, LayoutMode::Single);
    assert_eq!(change.after.active_agent_ids, vec![a]);
}

#[test]
fn an_agent_of_another_workspace_changes_nothing() {
    let (mut store, workspace, [a, ..]) = trio();
    let other = knot_core::Workspace { id:        Uuid::new_v4(),
                                       name:      "other".into(),
                                       color_hex: "#000000".into(),
                                       agent_ids: Vec::new(), };
    let other_id = other.id;
    store.add_workspace(other);
    let stranger = store.create("/tmp/x",
                                CreateOptions { workspace_id: Some(other_id),
                                                ..Default::default() });
    let before = store.workspace_ui(workspace);

    let change = store.select_in_layout(workspace, stranger, false);

    assert!(!change.changed());
    assert_eq!(store.workspace_ui(workspace), before);
    assert_eq!(before.active_agent_ids, vec![a]);
}

#[test]
fn choosing_a_mode_fills_from_the_sidebar_order() {
    let (mut store, workspace, [a, b, c]) = trio();
    store.set_workspace_layout(workspace, LayoutMode::Single, vec![b]);

    let change = store.set_layout_mode(workspace, LayoutMode::ThreePane);

    assert_eq!(change.after.layout_mode, LayoutMode::ThreePane);
    assert_eq!(change.after.active_agent_ids, vec![b, a, c]);
    assert_eq!(change.placed(), vec![a, c]);
    assert_eq!(store.workspace_ui(workspace), change.after);
}

#[test]
fn focusing_a_pane_sets_the_index_and_ignores_one_that_is_not_shown() {
    let (mut store, workspace, [a, b, _]) = trio();
    store.set_workspace_layout(workspace, LayoutMode::SplitVertical, vec![a, b]);

    let change = store.focus_layout_pane(workspace, 1);
    assert_eq!(change.focused(), Some(b));
    assert_eq!(store.workspace_ui(workspace).focused_pane_index, 1);

    let change = store.focus_layout_pane(workspace, 2);
    assert!(!change.changed());
    assert_eq!(store.workspace_ui(workspace).focused_pane_index, 1);
}

#[test]
fn an_action_that_changes_nothing_does_not_create_an_entry() {
    let mut store = AgentStore::new();
    let workspace = knot_core::Workspace { id:        Uuid::new_v4(),
                                           name:      "empty".into(),
                                           color_hex: "#000000".into(),
                                           agent_ids: Vec::new(), };
    let id = workspace.id;
    store.add_workspace(workspace);
    let change = store.focus_layout_pane(id, 0);
    assert!(!change.changed());
    assert!(!store.saved_workspace_ui().contains_key(&id));
}

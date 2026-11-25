//! Companions and removal in a window with a layout (`workspace-layouts`,
//! "A new companion opens beside its creator" and "A companion whose owner is
//! not shown"): the store lays the
//! companion out, and the window focuses it, shows it and saves the layout.
//!
//! New Shell Companion is dispatched as the action the menu bar and the
//! keyboard send. New Companion... is the editor's callback, which is
//! `show_created_companion`; the editor's form is not opened here. An agent
//! creating a companion over MCP never reaches the window, so that test only
//! lets a frame pass.

use gpui_kit::TestAppContext;
use knot_agents::CompanionOrigin;
use knot_agents::CreateOptions;
use knot_core::LayoutMode;
use uuid::Uuid;

use super::layout_test_support::Fixture;
use super::layout_test_support::Spec;
use super::layout_test_support::open;
use crate::agent_menu::AgentMenuNewShellCompanion;

/// A split of A and B with a third agent C, `focus` on pane `focus`.
fn split(focus: i32, cx: &mut TestAppContext) -> Fixture {
    open(&Spec { agents: &["A", "B", "C"],
                 companions: &[],
                 mode: LayoutMode::SplitVertical,
                 panes: &["A", "B"],
                 focus },
         cx)
}

/// What a companion created for `owner` with `origin` looks like to the
/// store, as the editor and the MCP `create-agent` tool create one.
fn create_companion(f: &Fixture, owner: &str, origin: CompanionOrigin) -> Uuid {
    let owner = f.id(owner);
    f.store.lock().create("~/companion",
                          CreateOptions { agent_type: Some("layout-test-inert".into()),
                                          created_by: Some(owner),
                                          is_companion: true,
                                          insert_after: Some(owner),
                                          companion_origin: Some(origin),
                                          ..Default::default() })
}

fn selected_id(f: &mut Fixture) -> Option<Uuid> {
    f.view.read_with(&f.window, |view, _| view.selected_agent)
}

fn show_created_companion(f: &mut Fixture, id: Uuid) {
    f.view.update(&mut f.window, |view, cx| {
              view.show_created_companion(id, cx)
          });
    f.window.run_until_parked();
}

#[gpui_kit::test]
fn a_shell_companion_opens_beside_its_owner_from_single(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::Single,
                             panes:      &["A"],
                             focus:      0, },
                     cx);
    f.press(AgentMenuNewShellCompanion);

    let ui = f.ui();
    assert_eq!(ui.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(ui.active_agent_ids.len(), 2);
    assert_eq!(ui.active_agent_ids[0], f.id("A"));
    assert_eq!(ui.focused_pane_index, 1, "the companion is focused");
    assert_eq!(selected_id(&mut f), Some(ui.active_agent_ids[1]));
}

#[gpui_kit::test]
fn a_shell_companion_joins_a_split_as_a_third_pane(cx: &mut TestAppContext) {
    let mut f = split(0, cx);
    f.press(AgentMenuNewShellCompanion);

    let ui = f.ui();
    assert_eq!(ui.layout_mode, LayoutMode::ThreePane);
    assert_eq!(&ui.active_agent_ids[..2], &[f.id("A"), f.id("B")]);
    assert_eq!(ui.focused_pane_index, 2);
    assert_eq!(selected_id(&mut f), Some(ui.active_agent_ids[2]));
}

#[gpui_kit::test]
fn the_layout_a_new_companion_makes_is_saved(cx: &mut TestAppContext) {
    let mut f = split(0, cx);
    f.press(AgentMenuNewShellCompanion);
    let companion = f.ui().active_agent_ids[2];
    let saved = std::fs::read_to_string(&f.document).expect("the UI-state document");
    assert!(saved.contains(&companion.to_string()),
            "the document names the companion's pane: {saved}");
}

#[gpui_kit::test]
fn a_user_companion_of_an_owner_that_is_not_shown_makes_an_owner_and_companion_split(cx: &mut TestAppContext)
{
    // C is in no pane; the user creates a companion for it.
    let mut f = split(0, cx);
    let companion = create_companion(&f, "C", CompanionOrigin::User);
    show_created_companion(&mut f, companion);

    let ui = f.ui();
    assert_eq!(ui.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(ui.active_agent_ids, [f.id("C"), companion]);
    assert_eq!(ui.focused_pane_index, 1);
    assert_eq!(selected_id(&mut f), Some(companion));
    // Showing the owner in a pane was the user's action, so it starts.
    let owner = f.id("C");
    assert!(f.store
             .lock()
             .agent(owner)
             .is_some_and(|agent| agent.activated),
            "the owner the companion brought into view is started");
}

#[gpui_kit::test]
fn a_user_companion_from_the_editor_opens_beside_a_shown_owner(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::Single,
                             panes:      &["A"],
                             focus:      0, },
                     cx);
    let companion = create_companion(&f, "A", CompanionOrigin::User);
    show_created_companion(&mut f, companion);

    assert_eq!(f.ui().layout_mode, LayoutMode::SplitVertical);
    assert_eq!(f.ui().active_agent_ids, [f.id("A"), companion]);
    assert_eq!(selected_id(&mut f), Some(companion));
}

#[gpui_kit::test]
fn an_mcp_companion_leaves_the_layout_alone_when_its_owner_is_not_shown(cx: &mut TestAppContext) {
    let mut f = split(1, cx);
    let before = f.ui();
    let companion = create_companion(&f, "C", CompanionOrigin::Other);
    f.view.update(&mut f.window, |_, cx| cx.notify());
    f.window.run_until_parked();

    assert_eq!(f.ui(), before);
    assert_eq!(f.selected().as_deref(),
               Some("B"),
               "the user's pane keeps focus");
    assert!(f.store.lock().agent(companion).is_some(),
            "the companion exists, in the sidebar only");
}

#[gpui_kit::test]
fn an_mcp_companion_opens_beside_a_shown_owner_and_takes_focus(cx: &mut TestAppContext) {
    for (panes, mode) in [(&["A"][..], LayoutMode::Single),
                          (&["A", "B"][..], LayoutMode::SplitVertical)]
    {
        let mut f = open(&Spec { agents: &["A", "B"],
                                 companions: &[],
                                 mode,
                                 panes,
                                 focus: 0 },
                         cx);
        let companion = create_companion(&f, "A", CompanionOrigin::Other);
        f.view.update(&mut f.window, |_, cx| cx.notify());
        f.window.run_until_parked();

        let ui = f.ui();
        assert_eq!(ui.active_agent_ids.last(), Some(&companion));
        assert_eq!(ui.focused_pane_index,
                   i32::try_from(ui.active_agent_ids.len() - 1).unwrap());
        assert_eq!(selected_id(&mut f),
                   Some(companion),
                   "the selection follows the store's focus");
    }
}

/// Opens `owner`'s MCP handover terminal and reveals it, as the MCP pane's
/// action does (`render/mcp_pane.rs`): the store opens it, the window shows it.
///
/// The terminal is a shell, which revealing would start - a PTY thread the
/// test scheduler rejects as nondeterministic. So before the reveal it takes
/// the fixture's inert type, the one every other agent here has. Where a
/// companion goes depends on its owner and origin, never on its type.
fn open_and_reveal_handover(f: &mut Fixture, owner: &str) -> Uuid {
    let owner = f.id(owner);
    let terminal = f.store
                    .lock()
                    .open_mcp_handover(knot_agents::McpHandoverRequest { owner,
                                                                         server: "linear",
                                                                         command: "true",
                                                                         name: "MCP" })
                    .expect("the handover terminal opens")
                    .id();
    {
        let mut store = f.store.lock();
        let agent = store.agent(terminal).expect("the terminal").clone();
        store.edit(terminal,
                   knot_agents::EditRequest { name: agent.name,
                                              avatar: agent.avatar,
                                              agent_type: Some("layout-test-inert".into()),
                                              activation_mode: agent.activation_mode,
                                              ..Default::default() })
             .expect("the terminal takes the inert type");
    }
    f.view
     .update(&mut f.window, |view, cx| view.reveal_agent(terminal, cx));
    f.window.run_until_parked();
    terminal
}

/// #500: revealing the handover terminal from a split keeps the split - it
/// opens beside its owner as a user's companion, not alone by rule R3.
#[gpui_kit::test]
fn revealing_the_handover_terminal_keeps_a_split(cx: &mut TestAppContext) {
    let mut f = split(0, cx);
    let terminal = open_and_reveal_handover(&mut f, "A");

    let ui = f.ui();
    assert_eq!(ui.layout_mode, LayoutMode::ThreePane);
    assert_eq!(ui.active_agent_ids, [f.id("A"), f.id("B"), terminal]);
    assert_eq!(selected_id(&mut f), Some(terminal));
}

/// From a single pane it opens beside its owner too, as the user's own
/// companions do ("A new companion opens beside its creator", from single).
#[gpui_kit::test]
fn revealing_the_handover_terminal_from_single_shows_it_beside_its_owner(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::Single,
                             panes:      &["A"],
                             focus:      0, },
                     cx);
    let terminal = open_and_reveal_handover(&mut f, "A");

    let ui = f.ui();
    assert_eq!(ui.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(ui.active_agent_ids, [f.id("A"), terminal]);
    assert_eq!(selected_id(&mut f), Some(terminal));
}

/// Opened again after it left the layout, the same terminal comes back
/// beside its owner rather than replacing the split.
#[gpui_kit::test]
fn reopening_a_handover_terminal_that_left_the_layout_keeps_the_split(cx: &mut TestAppContext) {
    let mut f = split(0, cx);
    let terminal = open_and_reveal_handover(&mut f, "A");
    // The user puts the split back without it.
    let (a, b) = (f.id("A"), f.id("B"));
    f.store
     .lock()
     .set_workspace_layout(f.workspace, LayoutMode::SplitVertical, vec![a, b]);
    f.store.lock().set_workspace_focused_pane(f.workspace, 0);

    let again = open_and_reveal_handover(&mut f, "A");

    assert_eq!(again, terminal, "the same terminal");
    let ui = f.ui();
    assert_eq!(ui.layout_mode, LayoutMode::ThreePane);
    assert_eq!(ui.active_agent_ids, [f.id("A"), f.id("B"), terminal]);
}

/// #507 review: the layout the handover terminal opens is saved, not only
/// shown. The reveal after it finds the terminal already in a pane and
/// changes nothing, so the window's open has to save it, or a quit straight
/// after restores the split from before.
///
/// Through the window's own `open_mcp_handover`, which needs an owner whose
/// type has an MCP flow; `claude` is one. The owner is passive and is never
/// revealed here, so nothing launches.
#[gpui_kit::test]
fn the_layout_the_handover_terminal_opens_is_saved(cx: &mut TestAppContext) {
    let mut f = split(0, cx);
    let owner = f.id("A");
    {
        let mut store = f.store.lock();
        let agent = store.agent(owner).expect("A").clone();
        store.edit(owner,
                   knot_agents::EditRequest { name: agent.name,
                                              avatar: agent.avatar,
                                              agent_type:
                                                  Some(knot_core::agent_type::CLAUDE.into()),
                                              activation_mode: agent.activation_mode,
                                              ..Default::default() })
             .expect("A becomes a claude agent");
    }

    let terminal = f.view
                    .update(&mut f.window, |view, cx| {
                        view.open_mcp_handover(owner, "linear", cx)
                    })
                    .expect("a claude agent has an MCP flow");
    f.window.run_until_parked();

    let live = f.ui();
    assert_eq!(live.active_agent_ids, [owner, f.id("B"), terminal]);
    let saved: std::collections::BTreeMap<Uuid, knot_core::WorkspaceUiState> =
        serde_json::from_str(&std::fs::read_to_string(&f.document).expect("the UI-state document"))
            .expect("the UI-state document parses");
    assert_eq!(saved[&f.workspace].active_agent_ids, live.active_agent_ids,
               "the saved layout is the live one");
}

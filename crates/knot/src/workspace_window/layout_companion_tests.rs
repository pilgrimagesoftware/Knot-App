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

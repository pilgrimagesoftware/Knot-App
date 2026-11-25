//! Removal in a window with a layout (`workspace-layouts`, "Removal
//! collapses the layout"): the store collapses the layout, and the window
//! leaves `selected_agent` on a shown agent and saves the result.

use gpui_kit::TestAppContext;
use knot_core::LayoutMode;

use super::layout_test_support::Spec;
use super::layout_test_support::open;

#[gpui_kit::test]
fn removing_the_focused_agent_of_three_panes_collapses_to_a_split(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B", "C"],
                             companions: &[],
                             mode:       LayoutMode::ThreePane,
                             panes:      &["A", "B", "C"],
                             focus:      1, },
                     cx);
    let (b, removed_text) = (f.id("B"), f.id("B").to_string());
    f.view
     .update(&mut f.window, |view, cx| view.remove_agent(b, cx));
    f.window.run_until_parked();

    assert_eq!(f.panes(), ["A", "C"]);
    assert_eq!(f.ui().layout_mode, LayoutMode::SplitVertical);
    assert_eq!(f.ui().focused_pane_index, 1, "B's place went to C");
    assert_eq!(f.selected().as_deref(),
               Some("C"),
               "selection is on a shown agent");
    let saved = std::fs::read_to_string(&f.document).expect("the UI-state document");
    assert!(!saved.contains(&removed_text),
            "the saved layout lost B: {saved}");
}

#[gpui_kit::test]
fn removing_another_pane_leaves_the_focused_agent_selected(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B", "C"],
                             companions: &[],
                             mode:       LayoutMode::ThreePane,
                             panes:      &["A", "B", "C"],
                             focus:      1, },
                     cx);
    let a = f.id("A");
    f.view
     .update(&mut f.window, |view, cx| view.remove_agent(a, cx));
    f.window.run_until_parked();

    assert_eq!(f.panes(), ["B", "C"]);
    assert_eq!(f.ui().focused_pane_index, 0, "B kept focus");
    assert_eq!(f.selected().as_deref(), Some("B"));
}

#[gpui_kit::test]
fn a_shell_companion_exiting_leaves_its_owner_shown_alone(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[("A", "S")],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "S"],
                             focus:      1, },
                     cx);
    // What the repaint poll does for an exited shell.
    let shell = f.id("S");
    f.view.update(&mut f.window, |view, cx| {
              view.finish_mcp_handover(shell);
              view.remove_agent(shell, cx);
          });
    f.window.run_until_parked();

    assert_eq!(f.panes(), ["A"]);
    assert_eq!(f.ui().layout_mode, LayoutMode::Single);
    assert_eq!(f.selected().as_deref(), Some("A"));
}

#[gpui_kit::test]
fn removing_the_only_shown_agent_shows_the_first_remaining_one(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::Single,
                             panes:      &["A"],
                             focus:      0, },
                     cx);
    let a = f.id("A");
    f.view
     .update(&mut f.window, |view, cx| view.remove_agent(a, cx));
    f.window.run_until_parked();

    assert_eq!(f.panes(), ["B"]);
    assert_eq!(f.selected().as_deref(), Some("B"));
}

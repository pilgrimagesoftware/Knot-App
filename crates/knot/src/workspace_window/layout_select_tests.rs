//! Selecting an agent in a window with a layout: every path that chooses an
//! agent - a sidebar click with and without the platform's secondary
//! modifier, the agent-number shortcut - goes through the layout rules R1 to
//! R4 of `workspace-layouts`, and the window's selection ends up on the
//! focused pane's agent.
//!
//! The ⌘-click is simulated with `Modifiers::secondary_key()`, which is what
//! the product reads (`ClickEvent::modifiers().secondary()`): ⌘ on macOS and
//! Ctrl elsewhere. A literal `platform` modifier would match only on a Mac.

use gpui_kit::Modifiers;
use gpui_kit::TestAppContext;
use knot_core::LayoutMode;

use super::layout_test_support::Spec;
use super::layout_test_support::open;
use crate::keymap::SelectAgent2;

#[gpui_kit::test]
fn a_click_replaces_the_focused_pane(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B", "C"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      1, },
                     cx);
    f.click_row("C", Modifiers::none());
    assert_eq!(f.panes(), ["A", "C"]);
    assert_eq!(f.ui().focused_pane_index, 1);
    assert_eq!(f.selected().as_deref(), Some("C"));
}

#[gpui_kit::test]
fn a_click_on_a_shown_agent_only_moves_focus(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B", "C"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    f.click_row("B", Modifiers::none());
    assert_eq!(f.panes(), ["A", "B"]);
    assert_eq!(f.ui().layout_mode, LayoutMode::SplitVertical);
    assert_eq!(f.ui().focused_pane_index, 1);
    assert_eq!(f.selected().as_deref(), Some("B"));
}

#[gpui_kit::test]
fn a_click_on_a_shown_companion_leaves_the_layout(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[("A", "S")],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "S"],
                             focus:      0, },
                     cx);
    f.click_row("S", Modifiers::none());
    assert_eq!(f.panes(), ["A", "S"]);
    assert_eq!(f.ui().focused_pane_index, 1);
    assert_eq!(f.selected().as_deref(), Some("S"));
}

#[gpui_kit::test]
fn command_click_skips_only_the_companion_layout(cx: &mut TestAppContext) {
    // Owner A and its companion S are shown with A in pane 0; B is another
    // owner, with companion T.
    let spec = Spec { agents:     &["A", "B"],
                      companions: &[("A", "S"), ("B", "T")],
                      mode:       LayoutMode::SplitVertical,
                      panes:      &["A", "S"],
                      focus:      0, };
    let mut f = open(&spec, cx);
    f.click_row("B", Modifiers::secondary_key());
    assert_eq!(f.panes(),
               ["B"],
               "R1 is skipped, so R3 shows B alone rather than B with T");
    assert_eq!(f.ui().layout_mode, LayoutMode::Single);
    assert_eq!(f.selected().as_deref(), Some("B"));
}

#[gpui_kit::test]
fn a_plain_click_on_an_owner_shows_its_companion_layout(cx: &mut TestAppContext) {
    let spec = Spec { agents:     &["A", "B"],
                      companions: &[("A", "S"), ("B", "T")],
                      mode:       LayoutMode::SplitVertical,
                      panes:      &["A", "S"],
                      focus:      0, };
    let mut f = open(&spec, cx);
    f.click_row("B", Modifiers::none());
    assert_eq!(f.panes(), ["B", "T"], "R1 applies without the modifier");
    assert_eq!(f.ui().focused_pane_index, 0);
}

#[gpui_kit::test]
fn the_modifier_of_the_other_platform_is_not_the_skip(cx: &mut TestAppContext) {
    // The check the keystroke rule asks for: the modifier that is not the
    // platform's secondary one must leave R1 in force.
    let spec = Spec { agents:     &["A", "B"],
                      companions: &[("A", "S"), ("B", "T")],
                      mode:       LayoutMode::SplitVertical,
                      panes:      &["A", "S"],
                      focus:      0, };
    let mut f = open(&spec, cx);
    let other = if cfg!(target_os = "macos") {
        Modifiers::control()
    }
    else {
        Modifiers::command()
    };
    f.click_row("B", other);
    assert_eq!(f.panes(), ["B", "T"]);
}

#[gpui_kit::test]
fn leaving_a_companion_layout_shows_the_clicked_agent_alone(cx: &mut TestAppContext) {
    let spec = Spec { agents:     &["A", "C"],
                      companions: &[("A", "S")],
                      mode:       LayoutMode::SplitVertical,
                      panes:      &["A", "S"],
                      focus:      0, };
    let mut f = open(&spec, cx);
    f.click_row("C", Modifiers::none());
    assert_eq!(f.panes(), ["C"]);
    assert_eq!(f.ui().layout_mode, LayoutMode::Single);
}

#[gpui_kit::test]
fn the_number_of_a_shown_agent_only_moves_focus(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B", "C"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    f.press(SelectAgent2);
    assert_eq!(f.panes(), ["A", "B"]);
    assert_eq!(f.ui().focused_pane_index, 1);
    assert_eq!(f.selected().as_deref(), Some("B"));
}

#[gpui_kit::test]
fn the_number_counts_companion_rows(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[("A", "S")],
                             mode:       LayoutMode::Single,
                             panes:      &["B"],
                             focus:      0, },
                     cx);
    // Rows are A, S, B: the second row is A's companion.
    f.press(SelectAgent2);
    assert_eq!(f.selected().as_deref(), Some("S"));
    assert_eq!(f.panes(), ["S"]);
}

#[gpui_kit::test]
fn choosing_an_agent_starts_it_and_restoring_does_not(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B", "C"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    assert!(!f.is_activated("A"),
            "a restored pane does not start its agent");
    assert!(!f.is_activated("B"));

    // C goes into pane 0, replacing A, and starts; B was not placed by the
    // click and stays as it was.
    f.click_row("C", Modifiers::none());
    assert!(f.is_activated("C"));
    assert!(!f.is_activated("B"));
}

#[gpui_kit::test]
fn a_click_that_changes_the_layout_saves_it(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B", "C"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    f.click_row("C", Modifiers::none());
    let saved = std::fs::read_to_string(&f.document).expect("the click wrote the document");
    let id = f.id("C").to_string();
    assert!(saved.contains(&id), "the saved layout names the new pane");
}

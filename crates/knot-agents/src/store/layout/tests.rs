//! One test per scenario of `workspace-layouts`, named after it, run against
//! the pure rules with no store or window.

use knot_core::consts::{
    WORKSPACE_SPLIT_RATIO_DEFAULT, WORKSPACE_SPLIT_RATIO_MAX, WORKSPACE_SPLIT_RATIO_MIN,
};
use knot_core::{LayoutMode, WorkspaceUiState};
use uuid::Uuid;

use super::{CompanionOrigin, add_companion, normalize, remove, select, set_mode};

/// Agents `a` to `h`, by letter, so a test reads like its scenario.
fn id(letter: char) -> Uuid {
    Uuid::from_u128(u128::from(letter))
}

fn ids(letters: &str) -> Vec<Uuid> {
    letters.chars().map(id).collect()
}

fn state(mode: LayoutMode, panes: &str, focus: i32) -> WorkspaceUiState {
    WorkspaceUiState { layout_mode: mode,
                       active_agent_ids: ids(panes),
                       focused_pane_index: focus,
                       ..Default::default() }
}

/// Which pane agent letters sit in, for comparing against a scenario.
fn shown(ui: &WorkspaceUiState) -> Vec<Uuid> {
    ui.active_agent_ids.clone()
}

/// A companion lookup in which each `(owner, companions)` pair says who owns
/// whom.
fn owners(pairs: &'static [(char, &'static str)]) -> impl Fn(Uuid) -> Vec<Uuid> {
    move |owner| {
        pairs.iter()
             .find(|(candidate, _)| id(*candidate) == owner)
             .map(|(_, companions)| ids(companions))
             .unwrap_or_default()
    }
}

fn nobody(_: Uuid) -> Vec<Uuid> {
    Vec::new()
}

// Changing the layout mode

#[test]
fn single_to_split_fills_from_the_sidebar() {
    let ui = state(LayoutMode::Single, "b", 0);
    let next = set_mode(&ui, &ids("abc"), LayoutMode::SplitVertical);
    assert_eq!(next.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(shown(&next), ids("ba"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn back_to_single_keeps_the_focused_agent() {
    let ui = state(LayoutMode::ThreePane, "abc", 2);
    let next = set_mode(&ui, &ids("abc"), LayoutMode::Single);
    assert_eq!(next.layout_mode, LayoutMode::Single);
    assert_eq!(shown(&next), ids("c"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn a_smaller_mode_drops_the_agents_past_its_panes_and_keeps_focus_on_its_agent() {
    let ui = state(LayoutMode::GridFourPane, "abcd", 1);
    let next = set_mode(&ui, &ids("abcd"), LayoutMode::SplitHorizontal);
    assert_eq!(shown(&next), ids("ab"));
    assert_eq!(next.focused_pane_index, 1);
}

#[test]
fn focus_goes_to_pane_zero_when_its_agent_is_dropped() {
    let ui = state(LayoutMode::GridFourPane, "abcd", 3);
    let next = set_mode(&ui, &ids("abcd"), LayoutMode::SplitVertical);
    assert_eq!(shown(&next), ids("ab"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn a_larger_mode_keeps_the_panes_in_order_and_the_ratios() {
    let ui = WorkspaceUiState { split_ratio: 0.3,
                                split_ratio_secondary: Some(0.6),
                                ..state(LayoutMode::SplitVertical, "ca", 1) };
    let next = set_mode(&ui, &ids("abcd"), LayoutMode::GridFourPane);
    assert_eq!(shown(&next), ids("cabd"));
    assert_eq!(next.focused_pane_index, 1);
    assert_eq!((next.split_ratio, next.split_ratio_secondary),
               (0.3, Some(0.6)));
}

// Selecting an agent from the sidebar

#[test]
fn replacing_the_focused_pane() {
    let ui = state(LayoutMode::SplitVertical, "ab", 1);
    let next = select(&ui, id('c'), false, nobody);
    assert_eq!(next.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(shown(&next), ids("ac"));
    assert_eq!(next.focused_pane_index, 1);
}

#[test]
fn focusing_an_agent_already_shown() {
    let ui = state(LayoutMode::SplitVertical, "ab", 0);
    let next = select(&ui, id('b'), false, nobody);
    assert_eq!(shown(&next), ids("ab"));
    assert_eq!(next.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(next.focused_pane_index, 1);
}

#[test]
fn clicking_a_companion_that_is_shown() {
    let ui = state(LayoutMode::SplitVertical, "as", 0);
    let next = select(&ui, id('s'), false, owners(&[('a', "s")]));
    assert_eq!(shown(&next), ids("as"));
    assert_eq!(next.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(next.focused_pane_index, 1);
}

#[test]
fn command_click_skips_only_the_companion_layout() {
    let ui = state(LayoutMode::SplitVertical, "as", 0);
    let lookup = owners(&[('a', "s"), ('b', "t")]);
    let next = select(&ui, id('b'), true, &lookup);
    assert_eq!(next.layout_mode, LayoutMode::Single);
    assert_eq!(shown(&next), ids("b"));
    // The same click without the modifier opens b's companion layout.
    let plain = select(&ui, id('b'), false, &lookup);
    assert_eq!(shown(&plain), ids("bt"));
}

#[test]
fn command_click_on_a_shown_agent_still_focuses_it() {
    let ui = state(LayoutMode::SplitVertical, "as", 1);
    let next = select(&ui, id('a'), true, owners(&[('a', "s")]));
    assert_eq!(shown(&next), ids("as"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn leaving_a_companion_layout() {
    let ui = state(LayoutMode::SplitVertical, "as", 0);
    let next = select(&ui, id('c'), false, owners(&[('a', "s")]));
    assert_eq!(next.layout_mode, LayoutMode::Single);
    assert_eq!(shown(&next), ids("c"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn an_owner_whose_pane_is_not_focused_is_selected_by_the_ordinary_rules() {
    // R1 needs pane 0 focused. With pane 1 focused, clicking an owner that is
    // not shown replaces the focused pane, as R4 says.
    let ui = state(LayoutMode::SplitVertical, "bc", 1);
    let next = select(&ui, id('a'), false, owners(&[('a', "s")]));
    assert_eq!(shown(&next), ids("ba"));
    assert_eq!(next.focused_pane_index, 1);
}

// Companion layout

#[test]
fn an_owner_with_two_companions() {
    let ui = state(LayoutMode::Single, "c", 0);
    let next = select(&ui, id('a'), false, owners(&[('a', "xy")]));
    assert_eq!(next.layout_mode, LayoutMode::ThreePane);
    assert_eq!(shown(&next), ids("axy"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn an_owner_with_one_companion_gets_a_split_and_with_many_the_first_three() {
    let ui = state(LayoutMode::Single, "c", 0);
    let one = select(&ui, id('a'), false, owners(&[('a', "x")]));
    assert_eq!((one.layout_mode, shown(&one)),
               (LayoutMode::SplitVertical, ids("ax")));
    let many = select(&ui, id('a'), false, owners(&[('a', "wxyz")]));
    assert_eq!((many.layout_mode, shown(&many)),
               (LayoutMode::GridFourPane, ids("awxy")));
}

#[test]
fn command_click_treats_an_owner_as_an_agent_without_companions() {
    let ui = state(LayoutMode::Single, "c", 0);
    let next = select(&ui, id('a'), true, owners(&[('a', "x")]));
    assert_eq!(shown(&next), ids("a"));
    assert_eq!(next.layout_mode, LayoutMode::Single);
}

// A new companion opens beside its creator

#[test]
fn a_new_companion_from_single() {
    let ui = state(LayoutMode::Single, "a", 0);
    let next = add_companion(&ui, id('a'), id('s'), CompanionOrigin::User);
    assert_eq!(next.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(shown(&next), ids("as"));
    assert_eq!(next.focused_pane_index, 1);
}

#[test]
fn a_new_companion_from_a_split_makes_three_panes() {
    let ui = state(LayoutMode::SplitHorizontal, "ab", 0);
    let next = add_companion(&ui, id('a'), id('s'), CompanionOrigin::Other);
    assert_eq!(next.layout_mode, LayoutMode::ThreePane);
    assert_eq!(shown(&next), ids("abs"));
    assert_eq!(next.focused_pane_index, 2);
}

#[test]
fn a_new_companion_from_three_panes_makes_a_grid() {
    let ui = state(LayoutMode::ThreePane, "abc", 1);
    let next = add_companion(&ui, id('b'), id('s'), CompanionOrigin::User);
    assert_eq!(next.layout_mode, LayoutMode::GridFourPane);
    assert_eq!(shown(&next), ids("abcs"));
    assert_eq!(next.focused_pane_index, 3);
}

#[test]
fn a_new_companion_from_a_full_grid() {
    let ui = state(LayoutMode::GridFourPane, "abcd", 0);
    let next = add_companion(&ui, id('a'), id('s'), CompanionOrigin::User);
    assert_eq!(next.layout_mode, LayoutMode::GridFourPane);
    assert_eq!(shown(&next), ids("abcs"));
    assert_eq!(next.focused_pane_index, 3);
}

#[test]
fn a_new_companion_when_the_owner_holds_the_last_pane() {
    let ui = state(LayoutMode::GridFourPane, "bcda", 3);
    let next = add_companion(&ui, id('a'), id('s'), CompanionOrigin::User);
    assert_eq!(shown(&next), ids("bcsa"));
    assert_eq!(next.focused_pane_index, 2);
}

// A companion whose owner is not shown

#[test]
fn created_from_the_row_of_an_owner_that_is_not_shown() {
    let ui = state(LayoutMode::SplitVertical, "bc", 0);
    let next = add_companion(&ui, id('a'), id('s'), CompanionOrigin::User);
    assert_eq!(next.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(shown(&next), ids("as"));
    assert_eq!(next.focused_pane_index, 1);
}

#[test]
fn created_over_mcp_while_the_user_looks_elsewhere() {
    let ui = state(LayoutMode::SplitVertical, "bc", 1);
    let next = add_companion(&ui, id('a'), id('s'), CompanionOrigin::Other);
    assert_eq!(next, ui);
}

// Removal collapses the layout

#[test]
fn grid_loses_a_pane() {
    let ui = state(LayoutMode::GridFourPane, "abcd", 2);
    let next = remove(&ui, &ids("abd"), id('c'));
    assert_eq!(next.layout_mode, LayoutMode::ThreePane);
    assert_eq!(shown(&next), ids("abd"));
    assert_eq!(next.focused_pane_index, 2);
}

#[test]
fn split_loses_a_pane() {
    let ui = state(LayoutMode::SplitVertical, "as", 1);
    let next = remove(&ui, &ids("a"), id('s'));
    assert_eq!(next.layout_mode, LayoutMode::Single);
    assert_eq!(shown(&next), ids("a"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn a_layout_left_with_two_panes_is_a_vertical_split_and_with_one_single() {
    let ui = state(LayoutMode::ThreePane, "abc", 0);
    let next = remove(&ui, &ids("bc"), id('a'));
    assert_eq!(next.layout_mode, LayoutMode::SplitVertical);
    assert_eq!(shown(&next), ids("bc"));
    let horizontal = state(LayoutMode::SplitHorizontal, "ab", 0);
    assert_eq!(remove(&horizontal, &ids("b"), id('a')).layout_mode,
               LayoutMode::Single);
}

#[test]
fn a_single_layout_whose_agent_is_removed_shows_the_first_remaining_agent() {
    let ui = state(LayoutMode::Single, "b", 0);
    let next = remove(&ui, &ids("acd"), id('b'));
    assert_eq!(shown(&next), ids("a"));
    assert_eq!(next.layout_mode, LayoutMode::Single);
}

#[test]
fn removing_an_agent_that_is_in_no_pane_changes_nothing() {
    let ui = state(LayoutMode::SplitVertical, "ab", 1);
    assert_eq!(remove(&ui, &ids("ab"), id('z')), ui);
}

// Moving an agent to another workspace: the old workspace's half is `remove`;
// the store test covers the target being left alone.

// The saved layout is restored

#[test]
fn a_saved_split_reopens() {
    let ui = WorkspaceUiState { split_ratio: 0.4,
                                split_ratio_secondary: Some(0.6),
                                ..state(LayoutMode::ThreePane, "abc", 1) };
    assert_eq!(normalize(&ui, &ids("abcd")), ui);
}

#[test]
fn a_saved_agent_was_removed() {
    let ui = state(LayoutMode::SplitVertical, "xa", 0);
    let next = normalize(&ui, &ids("a"));
    assert_eq!(next.layout_mode, LayoutMode::Single);
    assert_eq!(shown(&next), ids("a"));
    assert_eq!(next.focused_pane_index, 0);
}

#[test]
fn a_saved_grid_with_a_missing_agent_reduces_to_three_panes() {
    let ui = state(LayoutMode::GridFourPane, "abxd", 3);
    let next = normalize(&ui, &ids("abcd"));
    assert_eq!(next.layout_mode, LayoutMode::ThreePane);
    assert_eq!(shown(&next), ids("abd"));
    assert_eq!(next.focused_pane_index, 2,
               "clamped to the panes that remain");
}

#[test]
fn a_saved_layout_naming_nothing_that_exists_shows_the_first_agent() {
    let ui = state(LayoutMode::SplitVertical, "xy", 1);
    let next = normalize(&ui, &ids("bc"));
    assert_eq!((next.layout_mode, shown(&next)),
               (LayoutMode::Single, ids("b")));
}

#[test]
fn a_stale_focused_pane_is_clamped() {
    for stale in [5, -1] {
        let ui = state(LayoutMode::SplitVertical, "ab", stale);
        let next = normalize(&ui, &ids("ab"));
        assert_eq!(next.focused_pane_index, if stale < 0 { 0 } else { 1 });
    }
}

#[test]
fn a_saved_agent_in_two_panes_is_shown_once_and_extra_panes_are_dropped() {
    let twice = normalize(&state(LayoutMode::SplitVertical, "aa", 0), &ids("ab"));
    assert_eq!((twice.layout_mode, shown(&twice)),
               (LayoutMode::Single, ids("a")));
    let crowded = normalize(&state(LayoutMode::SplitVertical, "abc", 0), &ids("abc"));
    assert_eq!(shown(&crowded), ids("ab"));
}

#[test]
fn saved_ratios_are_clamped() {
    let ui = WorkspaceUiState { split_ratio: 0.1,
                                split_ratio_secondary: Some(0.9),
                                ..state(LayoutMode::ThreePane, "abc", 0) };
    let next = normalize(&ui, &ids("abc"));
    assert_eq!(next.split_ratio, WORKSPACE_SPLIT_RATIO_MIN);
    assert_eq!(next.split_ratio_secondary, Some(WORKSPACE_SPLIT_RATIO_MAX));
    let broken = WorkspaceUiState { split_ratio: f64::NAN,
                                    ..state(LayoutMode::Single, "a", 0) };
    assert_eq!(normalize(&broken, &ids("a")).split_ratio,
               WORKSPACE_SPLIT_RATIO_DEFAULT);
}

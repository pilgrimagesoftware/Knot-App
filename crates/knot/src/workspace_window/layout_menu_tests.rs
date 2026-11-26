//! The Layout menu: which modes it lists for how many agents, that every
//! label resolves in the catalogue, and what choosing a mode does to the
//! panes and to the agents placed in them (`workspace-layouts`, "The Layout
//! menu", "Changing the layout mode", "Showing an agent in a pane starts it").
//!
//! The menu's popup is built when it opens, so the lines it lists are
//! asserted on `layout_choices`, the list the popup is built from, and the
//! window tests assert that the button is drawn or not. Choosing a line
//! calls `set_layout_mode`, which the popup's click handler calls.

use gpui_kit::TestAppContext;
use knot_core::LayoutMode;

use super::layout_test_support::Spec;
use super::layout_test_support::open;
use super::render::layout_menu::icon;
use super::render::layout_menu::label_key;
use super::render::layout_menu::layout_choices;
use super::render::layout_menu::tooltip;

fn modes(agents: usize, current: LayoutMode) -> Vec<LayoutMode> {
    layout_choices(agents, current).into_iter()
                                   .map(|choice| choice.mode)
                                   .collect()
}

#[test]
fn one_agent_has_no_menu() {
    assert!(layout_choices(0, LayoutMode::Single).is_empty());
    assert!(layout_choices(1, LayoutMode::Single).is_empty());
}

#[test]
fn two_agents_list_single_and_both_splits() {
    assert_eq!(modes(2, LayoutMode::Single),
               [LayoutMode::Single,
                LayoutMode::SplitVertical,
                LayoutMode::SplitHorizontal]);
}

#[test]
fn three_panes_needs_three_agents_and_four_panes_four() {
    assert!(!modes(2, LayoutMode::Single).contains(&LayoutMode::ThreePane));
    assert!(modes(3, LayoutMode::Single).contains(&LayoutMode::ThreePane));
    assert!(!modes(3, LayoutMode::Single).contains(&LayoutMode::GridFourPane));
    assert_eq!(modes(4, LayoutMode::Single), LayoutMode::ALL.to_vec());
    assert_eq!(modes(9, LayoutMode::Single), LayoutMode::ALL.to_vec());
}

#[test]
fn only_the_current_mode_is_checked() {
    for current in LayoutMode::ALL {
        let checked: Vec<_> = layout_choices(4, *current).into_iter()
                                                         .filter(|choice| choice.checked)
                                                         .map(|choice| choice.mode)
                                                         .collect();
        assert_eq!(checked, [*current]);
    }
}

/// A key missing from the catalogue comes back as the key itself.
#[test]
fn every_label_key_resolves_in_the_catalogue() {
    let mut seen = Vec::new();
    for mode in LayoutMode::ALL {
        let key = label_key(*mode);
        let label = knot_core::l10n::t(key);
        assert_ne!(label, key, "{key} must resolve");
        assert!(!seen.contains(&label), "{key} repeats another label");
        seen.push(label);
    }
}

/// The button shows the current mode, so no two modes may share an icon.
/// `icon` is an exhaustive match: a new mode fails to compile without one.
#[test]
fn every_mode_has_its_own_icon() {
    let mut seen = Vec::new();
    for mode in LayoutMode::ALL {
        let path = icon(*mode).path();
        assert!(!seen.contains(&path),
                "{mode:?} repeats another mode's icon");
        seen.push(path);
    }
}

/// Every glyph the menu draws is in the bundled asset set, so a name that
/// resolves to no SVG fails here rather than drawing a blank button: each
/// mode's icon, and the check mark the menu draws beside the current one.
#[test]
fn every_glyph_the_menu_draws_is_bundled() {
    use gpui_kit::AssetSource as _;
    use gpui_kit::component::IconNamed as _;

    let check = gpui_kit::component::IconName::Check.path();
    let paths = LayoutMode::ALL.iter()
                               .map(|mode| icon(*mode).path())
                               .chain(std::iter::once(check));
    for path in paths {
        let bytes = crate::app_support::KnotAssets.load(&path).ok().flatten();
        assert!(bytes.is_some_and(|bytes| !bytes.is_empty()),
                "{path} is not in the bundled assets");
    }
}

/// The tooltip resolves and names the current mode.
#[test]
fn the_tooltip_names_the_current_mode() {
    for mode in LayoutMode::ALL {
        let text = tooltip(*mode);
        assert!(!text.contains("layout.menu_tooltip"),
                "the tooltip key must resolve");
        assert!(!text.contains("%{mode}"), "the mode must be substituted");
        assert!(text.contains(&knot_core::l10n::t(label_key(*mode))),
                "{text:?} must name {mode:?}");
    }
}

fn roster(names: &'static [&'static str], mode: LayoutMode, panes: &'static [&'static str])
          -> Spec<'static> {
    Spec { agents: names,
           companions: &[],
           mode,
           panes,
           focus: 0 }
}

#[gpui_kit::test]
fn one_agent_draws_no_button(cx: &mut TestAppContext) {
    let mut one = open(&roster(&["A"], LayoutMode::Single, &["A"]), cx);
    assert!(one.window.debug_bounds("layout-menu").is_none());
}

#[gpui_kit::test]
fn two_agents_draw_the_button(cx: &mut TestAppContext) {
    let mut two = open(&roster(&["A", "B"], LayoutMode::Single, &["A"]), cx);
    assert!(two.window.debug_bounds("layout-menu").is_some());
}

#[gpui_kit::test]
fn four_agents_draw_the_button(cx: &mut TestAppContext) {
    let mut four = open(&roster(&["A", "B", "C", "D"], LayoutMode::Single, &["A"]),
                        cx);
    assert!(four.window.debug_bounds("layout-menu").is_some());
}

#[gpui_kit::test]
fn the_dashboard_hides_the_button(cx: &mut TestAppContext) {
    let mut f = open(&roster(&["A", "B"], LayoutMode::Single, &["A"]), cx);
    f.view.update_in(&mut f.window, |view, _, cx| {
              view.view_mode = super::WorkspaceViewMode::Dashboard;
              cx.notify();
          });
    f.window.run_until_parked();
    assert!(f.window.debug_bounds("layout-menu").is_none());
}

fn choose(f: &mut super::layout_test_support::Fixture, mode: LayoutMode) {
    f.view
     .update_in(&mut f.window, |view, _, cx| view.set_layout_mode(mode, cx));
    f.window.run_until_parked();
}

#[gpui_kit::test]
fn filling_panes_starts_the_agents_and_single_keeps_the_focused_one(cx: &mut TestAppContext) {
    let mut f = open(&roster(&["A", "B", "C"], LayoutMode::Single, &["A"]), cx);
    assert!(!f.is_activated("B"));
    assert!(!f.is_activated("C"));

    choose(&mut f, LayoutMode::ThreePane);
    assert_eq!(f.panes(), ["A", "B", "C"]);
    assert!(f.is_activated("B"), "B was placed in a pane, so it starts");
    assert!(f.is_activated("C"), "C was placed in a pane, so it starts");
    assert_eq!(f.selected().as_deref(), Some("A"));

    // Focus the last pane, then back to one: only its agent stays.
    f.view
     .update_in(&mut f.window, |view, _, cx| view.focus_pane(2, cx));
    f.window.run_until_parked();
    assert_eq!(f.selected().as_deref(), Some("C"));
    choose(&mut f, LayoutMode::Single);
    assert_eq!(f.panes(), ["C"]);
    assert_eq!(f.selected().as_deref(), Some("C"));
    assert_eq!(f.ui().layout_mode, LayoutMode::Single);
}

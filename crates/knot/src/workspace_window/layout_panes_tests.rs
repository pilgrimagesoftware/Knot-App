//! The panes a layout draws (`workspace-layouts`, "Pane geometry" and "Four-
//! pane grid geometry", "Pane contents"), in a real window: where each pane
//! lands in the layout area, and that single mode draws the one pane it
//! always did.

use gpui_kit::Bounds;
use gpui_kit::Pixels;
use gpui_kit::TestAppContext;
use knot_core::LayoutMode;

use super::layout_test_support::Fixture;
use super::layout_test_support::Spec;
use super::layout_test_support::open;

fn four(mode: LayoutMode, panes: &'static [&'static str], cx: &mut TestAppContext) -> Fixture {
    open(&Spec { agents: &["A", "B", "C", "D"],
                 companions: &[],
                 mode,
                 panes,
                 focus: 0 },
         cx)
}

fn pane(f: &mut Fixture, index: usize) -> Bounds<Pixels> {
    let selector: &'static str = Box::leak(format!("layout-pane-{index}").into_boxed_str());
    f.window
     .debug_bounds(selector)
     .unwrap_or_else(|| panic!("{selector} was never painted"))
}

fn area(f: &mut Fixture) -> Bounds<Pixels> {
    f.window
     .debug_bounds("layout-area")
     .expect("a multi-pane layout draws its area")
}

/// `bounds` within half a pixel of the fractions `(x, y, w, h)` of `area`.
fn assert_fractions(bounds: Bounds<Pixels>, area: Bounds<Pixels>,
                    (x, y, w, h): (f32, f32, f32, f32)) {
    let near = |got: Pixels, want: f32| {
        assert!((f32::from(got) - want).abs() < 0.5, "{got:?} is not {want}");
    };
    let (aw, ah) = (f32::from(area.size.width), f32::from(area.size.height));
    near(bounds.origin.x - area.origin.x, x * aw);
    near(bounds.origin.y - area.origin.y, y * ah);
    near(bounds.size.width, w * aw);
    near(bounds.size.height, h * ah);
}

#[gpui_kit::test]
fn single_mode_draws_the_one_pane_it_always_did(cx: &mut TestAppContext) {
    let mut f = four(LayoutMode::Single, &["A"], cx);
    assert!(f.window.debug_bounds("layout-area").is_none());
    assert!(f.window.debug_bounds("layout-pane-0").is_none());
}

#[gpui_kit::test]
fn a_vertical_split_puts_pane_zero_on_the_left(cx: &mut TestAppContext) {
    let mut f = four(LayoutMode::SplitVertical, &["A", "B"], cx);
    let area = area(&mut f);
    assert_fractions(pane(&mut f, 0), area, (0., 0., 0.5, 1.));
    assert_fractions(pane(&mut f, 1), area, (0.5, 0., 0.5, 1.));
}

#[gpui_kit::test]
fn a_horizontal_split_puts_pane_zero_on_top(cx: &mut TestAppContext) {
    let mut f = four(LayoutMode::SplitHorizontal, &["A", "B"], cx);
    let area = area(&mut f);
    assert_fractions(pane(&mut f, 0), area, (0., 0., 1., 0.5));
    assert_fractions(pane(&mut f, 1), area, (0., 0.5, 1., 0.5));
}

#[gpui_kit::test]
fn three_panes_put_one_column_left_and_two_right(cx: &mut TestAppContext) {
    let mut f = four(LayoutMode::ThreePane, &["A", "B", "C"], cx);
    let area = area(&mut f);
    assert_fractions(pane(&mut f, 0), area, (0., 0., 0.5, 1.));
    assert_fractions(pane(&mut f, 1), area, (0.5, 0., 0.5, 0.5));
    assert_fractions(pane(&mut f, 2), area, (0.5, 0.5, 0.5, 0.5));
}

#[gpui_kit::test]
fn a_grid_fills_its_four_quarters_in_reading_order(cx: &mut TestAppContext) {
    let mut f = four(LayoutMode::GridFourPane, &["A", "B", "C", "D"], cx);
    let area = area(&mut f);
    assert_fractions(pane(&mut f, 0), area, (0., 0., 0.5, 0.5));
    assert_fractions(pane(&mut f, 1), area, (0.5, 0., 0.5, 0.5));
    assert_fractions(pane(&mut f, 2), area, (0., 0.5, 0.5, 0.5));
    assert_fractions(pane(&mut f, 3), area, (0.5, 0.5, 0.5, 0.5));
}

#[gpui_kit::test]
fn the_ratio_moves_the_panes(cx: &mut TestAppContext) {
    let mut f = four(LayoutMode::SplitVertical, &["A", "B"], cx);
    f.store
     .lock()
     .set_workspace_split_ratios(f.workspace, 0.3, None);
    f.view.update(&mut f.window, |_, cx| cx.notify());
    f.window.run_until_parked();
    let area = area(&mut f);
    assert_fractions(pane(&mut f, 0), area, (0., 0., 0.3, 1.));
    assert_fractions(pane(&mut f, 1), area, (0.3, 0., 0.7, 1.));
}

fn split(cx: &mut TestAppContext) -> Fixture {
    open(&Spec { agents:     &["A", "B", "C"],
                 companions: &[],
                 mode:       LayoutMode::SplitVertical,
                 panes:      &["A", "B"],
                 focus:      0, },
         cx)
}

#[gpui_kit::test]
fn each_pane_of_a_multi_pane_layout_shows_its_agents_name(cx: &mut TestAppContext) {
    let mut f = split(cx);
    assert!(f.window.debug_bounds("layout-pane-header-0").is_some());
    assert!(f.window.debug_bounds("layout-pane-header-1").is_some());
    let header = f.window.debug_bounds("layout-pane-header-0").unwrap();
    let pane = pane(&mut f, 0);
    assert_eq!(header.origin, pane.origin, "the name is above the content");
}

#[gpui_kit::test]
fn a_single_layout_has_no_pane_header(cx: &mut TestAppContext) {
    let mut f = four(LayoutMode::Single, &["A"], cx);
    assert!(f.window.debug_bounds("layout-pane-header-0").is_none());
}

#[gpui_kit::test]
fn only_the_unfocused_panes_are_dimmed(cx: &mut TestAppContext) {
    let mut f = split(cx);
    assert!(f.window.debug_bounds("layout-pane-dim-0").is_none(),
            "pane 0 is focused");
    let dim = f.window
               .debug_bounds("layout-pane-dim-1")
               .expect("pane 1 is dimmed");
    assert_eq!(dim, pane(&mut f, 1), "the overlay covers the whole pane");
}

#[gpui_kit::test]
fn clicking_a_pane_selects_its_agent_and_takes_keyboard_focus(cx: &mut TestAppContext) {
    let mut f = split(cx);
    // A is running, so its pane has a composer to focus; B is stopped, and
    // starts when the click focuses it.
    f.store.lock().set_activated(f.id("A"), true);
    f.view.update(&mut f.window, |_, cx| cx.notify());
    f.window.run_until_parked();
    assert!(f.composer_focused("A"),
            "the focused pane's input has focus");
    assert!(f.row_highlighted("A"));

    let inside = pane(&mut f, 1).center();
    f.window.simulate_click(inside, gpui_kit::Modifiers::none());
    f.window.run_until_parked();

    assert_eq!(f.selected().as_deref(), Some("B"));
    assert_eq!(f.ui().focused_pane_index, 1);
    assert!(f.row_highlighted("B"));
    assert!(!f.row_highlighted("A"));
    assert!(f.composer_focused("B"),
            "keyboard focus moved with the pane");
    assert!(f.window.debug_bounds("layout-pane-dim-0").is_some(),
            "the pane that lost focus is dimmed");
    assert!(f.window.debug_bounds("layout-pane-dim-1").is_none());
    assert!(f.is_activated("B"),
            "focusing a pane selects it, which starts it");
}

#[gpui_kit::test]
fn a_click_in_the_focused_pane_changes_nothing(cx: &mut TestAppContext) {
    let mut f = split(cx);
    let before = std::fs::read_to_string(&f.document).ok();
    let inside = pane(&mut f, 0).center();
    f.window.simulate_click(inside, gpui_kit::Modifiers::none());
    f.window.run_until_parked();
    assert_eq!(f.ui().focused_pane_index, 0);
    assert!(!f.is_activated("A"),
            "a click that moves nothing starts nothing");
    assert_eq!(std::fs::read_to_string(&f.document).ok(), before);
}

#[gpui_kit::test]
fn grabbing_a_divider_does_not_move_the_focus(cx: &mut TestAppContext) {
    let mut f = split(cx);
    let grab = f.window
                .debug_bounds("layout-divider-primary")
                .expect("a split draws its divider")
                .center();
    f.window.simulate_mouse_down(grab,
                                 gpui_kit::MouseButton::Left,
                                 gpui_kit::Modifiers::none());
    f.window.simulate_mouse_up(grab,
                               gpui_kit::MouseButton::Left,
                               gpui_kit::Modifiers::none());
    f.window.run_until_parked();
    assert_eq!(f.ui().focused_pane_index, 0);
    assert_eq!(f.selected().as_deref(), Some("A"));
}

/// A layout the store changed without the window - an agent creating a
/// companion over MCP is one way - moves the focus, and the selection follows
/// it on the next frame, so `selected_agent` is still the focused pane's.
#[gpui_kit::test]
fn a_layout_changed_by_the_store_brings_the_selection_with_it(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    let owner = f.id("A");
    let shell = f.store.lock().create_shell_companion(owner).unwrap();
    f.view.update(&mut f.window, |_, cx| cx.notify());
    f.window.run_until_parked();

    assert_eq!(f.ui().layout_mode, LayoutMode::ThreePane);
    let selected = f.view.read_with(&f.window, |view, _| view.selected_agent);
    assert_eq!(selected,
               Some(shell),
               "the new companion is the focused pane");
}

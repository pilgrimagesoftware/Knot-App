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

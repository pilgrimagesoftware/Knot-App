//! The geometry of each layout, at the centre ratio, off-centre, and with
//! ratios past the bounds a divider may reach.

use gpui_kit::Bounds;
use gpui_kit::Pixels;
use gpui_kit::Size;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;
use knot_core::LayoutMode;

use super::DividerId;
use super::Orientation;
use super::divider_rects;
use super::dragged_ratio;
use super::pane_rects;

const AREA: (f32, f32) = (1000., 800.);

fn area() -> Size<Pixels> {
    size(px(AREA.0), px(AREA.1))
}

/// A rectangle as `[x, y, width, height]`, to compare with a tolerance: the
/// ratios are `f64` and the geometry is `f32`.
fn rect(bounds: Bounds<Pixels>) -> [f32; 4] {
    [bounds.origin.x.into(),
     bounds.origin.y.into(),
     bounds.size.width.into(),
     bounds.size.height.into()]
}

#[track_caller]
fn assert_rect(actual: Bounds<Pixels>, expected: [f32; 4]) {
    let actual = rect(actual);
    assert!(actual.iter()
                  .zip(expected)
                  .all(|(a, e)| (a - e).abs() < 0.01),
            "{actual:?} != {expected:?}");
}

#[track_caller]
fn assert_panes(mode: LayoutMode, r: f64, s: f64, expected: &[[f32; 4]]) {
    let panes = pane_rects(mode, r, s, area());
    assert_eq!(panes.len(), expected.len(), "{mode:?} pane count");
    for (pane, expected) in panes.into_iter().zip(expected) {
        assert_rect(pane, *expected);
    }
}

/// A divider's hit area, as `[x, y, width, height]`.
#[track_caller]
fn assert_dividers(mode: LayoutMode, r: f64, s: f64, expected: &[(DividerId, [f32; 4])]) {
    let found = divider_rects(mode, r, s, area());
    assert_eq!(found.len(), expected.len(), "{mode:?} divider count");
    for (found, (id, hit)) in found.into_iter().zip(expected) {
        assert_eq!(found.divider.id, *id);
        assert_rect(found.hit, *hit);
    }
}

#[test]
fn every_mode_has_as_many_panes_as_it_claims() {
    for mode in [LayoutMode::Single,
                 LayoutMode::SplitVertical,
                 LayoutMode::SplitHorizontal,
                 LayoutMode::ThreePane,
                 LayoutMode::GridFourPane]
    {
        assert_eq!(pane_rects(mode, 0.5, 0.5, area()).len(), mode.pane_count());
    }
}

#[test]
fn single_fills_the_area_and_has_no_divider() {
    assert_panes(LayoutMode::Single, 0.5, 0.5, &[[0., 0., 1000., 800.]]);
    assert!(divider_rects(LayoutMode::Single, 0.5, 0.5, area()).is_empty());
}

#[test]
fn split_vertical_at_the_centre_and_off_it() {
    assert_panes(LayoutMode::SplitVertical,
                 0.5,
                 0.5,
                 &[[0., 0., 500., 800.], [500., 0., 500., 800.]]);
    assert_panes(LayoutMode::SplitVertical,
                 0.3,
                 0.5,
                 &[[0., 0., 300., 800.], [300., 0., 700., 800.]]);
    assert_panes(LayoutMode::SplitVertical,
                 0.7,
                 0.5,
                 &[[0., 0., 700., 800.], [700., 0., 300., 800.]]);
    // The 12px hit area is centred on the line at x = r * width.
    assert_dividers(LayoutMode::SplitVertical,
                    0.3,
                    0.5,
                    &[(DividerId::Primary, [294., 0., 12., 800.])]);
}

#[test]
fn split_horizontal_at_the_centre_and_off_it() {
    assert_panes(LayoutMode::SplitHorizontal,
                 0.5,
                 0.5,
                 &[[0., 0., 1000., 400.], [0., 400., 1000., 400.]]);
    assert_panes(LayoutMode::SplitHorizontal,
                 0.7,
                 0.5,
                 &[[0., 0., 1000., 560.], [0., 560., 1000., 240.]]);
    assert_dividers(LayoutMode::SplitHorizontal,
                    0.3,
                    0.5,
                    &[(DividerId::Primary, [0., 234., 1000., 12.])]);
}

/// Pane 0 is the full-height left column; the right column is split at `s`.
#[test]
fn three_pane_at_the_centre_and_off_it() {
    assert_panes(LayoutMode::ThreePane,
                 0.5,
                 0.5,
                 &[[0., 0., 500., 800.],
                   [500., 0., 500., 400.],
                   [500., 400., 500., 400.]]);
    assert_panes(LayoutMode::ThreePane,
                 0.3,
                 0.7,
                 &[[0., 0., 300., 800.],
                   [300., 0., 700., 560.],
                   [300., 560., 700., 240.]]);
}

/// The secondary divider belongs to the right column: it starts at the
/// primary divider rather than at the area's left edge.
#[test]
fn three_pane_secondary_divider_spans_the_right_column_only() {
    assert_dividers(LayoutMode::ThreePane,
                    0.3,
                    0.7,
                    &[(DividerId::Primary, [294., 0., 12., 800.]),
                      (DividerId::Secondary, [300., 554., 700., 12.])]);
}

#[test]
fn grid_at_the_centre_and_off_it() {
    assert_panes(LayoutMode::GridFourPane,
                 0.5,
                 0.5,
                 &[[0., 0., 500., 400.],
                   [500., 0., 500., 400.],
                   [0., 400., 500., 400.],
                   [500., 400., 500., 400.]]);
    assert_panes(LayoutMode::GridFourPane,
                 0.3,
                 0.7,
                 &[[0., 0., 300., 560.],
                   [300., 0., 700., 560.],
                   [0., 560., 300., 240.],
                   [300., 560., 700., 240.]]);
}

/// Both grid dividers run the whole way, so the rows line up across columns.
#[test]
fn grid_dividers_run_full_length() {
    assert_dividers(LayoutMode::GridFourPane,
                    0.3,
                    0.7,
                    &[(DividerId::Primary, [294., 0., 12., 800.]),
                      (DividerId::Secondary, [0., 554., 1000., 12.])]);
}

#[test]
fn divider_orientation_follows_the_layout() {
    let orientation = |mode| divider_rects(mode, 0.5, 0.5, area())[0].divider.orientation;
    assert_eq!(orientation(LayoutMode::SplitVertical),
               Orientation::Vertical);
    assert_eq!(orientation(LayoutMode::SplitHorizontal),
               Orientation::Horizontal);
    let three = divider_rects(LayoutMode::ThreePane, 0.5, 0.5, area());
    assert_eq!(three[0].divider.orientation, Orientation::Vertical);
    assert_eq!(three[1].divider.orientation, Orientation::Horizontal);
}

/// 0.1 and 0.9 come out at the spec's 0.25 and 0.75.
#[test]
fn ratios_past_the_bounds_are_clamped() {
    assert_panes(LayoutMode::SplitVertical,
                 0.1,
                 0.5,
                 &[[0., 0., 250., 800.], [250., 0., 750., 800.]]);
    assert_panes(LayoutMode::SplitVertical,
                 0.9,
                 0.5,
                 &[[0., 0., 750., 800.], [750., 0., 250., 800.]]);
    assert_panes(LayoutMode::GridFourPane,
                 0.1,
                 0.9,
                 &[[0., 0., 250., 600.],
                   [250., 0., 750., 600.],
                   [0., 600., 250., 200.],
                   [250., 600., 750., 200.]]);
    assert_dividers(LayoutMode::ThreePane,
                    0.9,
                    0.1,
                    &[(DividerId::Primary, [744., 0., 12., 800.]),
                      (DividerId::Secondary, [750., 194., 250., 12.])]);
    assert_eq!(super::clamp_ratio(0.1),
               knot_core::consts::WORKSPACE_SPLIT_RATIO_MIN);
    assert_eq!(super::clamp_ratio(0.9),
               knot_core::consts::WORKSPACE_SPLIT_RATIO_MAX);
}

/// Panes tile the area exactly: nothing overlaps and nothing is left over.
#[test]
fn panes_cover_the_area() {
    for mode in [LayoutMode::SplitVertical,
                 LayoutMode::SplitHorizontal,
                 LayoutMode::ThreePane,
                 LayoutMode::GridFourPane]
    {
        let total: f32 = pane_rects(mode, 0.3, 0.7, area()).iter()
                                                           .map(|pane| {
                                                               f32::from(pane.size.width)
                                                               * f32::from(pane.size.height)
                                                           })
                                                           .sum();
        assert!((total - AREA.0 * AREA.1).abs() < 1.0, "{mode:?}: {total}");
    }
}

#[test]
fn the_origin_is_the_areas_own() {
    let panes = pane_rects(LayoutMode::SplitVertical, 0.5, 0.5, area());
    assert_eq!(panes[0].origin, point(px(0.), px(0.)));
}

#[test]
fn a_drag_moves_the_ratio_by_the_share_of_the_extent_travelled() {
    let ratio = dragged_ratio(0.5, px(100.), px(1000.));
    assert!((ratio - 0.6).abs() < 1e-6, "{ratio}");
    let ratio = dragged_ratio(0.5, px(-200.), px(800.));
    assert!((ratio - 0.25).abs() < 1e-6, "{ratio}");
}

#[test]
fn a_drag_past_a_bound_stops_at_it() {
    assert_eq!(dragged_ratio(0.5, px(-900.), px(1000.)),
               knot_core::consts::WORKSPACE_SPLIT_RATIO_MIN);
    assert_eq!(dragged_ratio(0.5, px(900.), px(1000.)),
               knot_core::consts::WORKSPACE_SPLIT_RATIO_MAX);
}

#[test]
fn a_drag_in_an_unmeasured_area_leaves_the_ratio() {
    assert_eq!(dragged_ratio(0.4, px(50.), px(0.)), 0.4);
}

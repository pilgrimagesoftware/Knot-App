//! Where a layout puts its panes and dividers: two pure functions over the
//! mode, the two ratios and the size of the layout area
//! (`workspace-layouts`, "Pane geometry" and "Four-pane grid geometry").
//!
//! Rectangles are relative to the layout area's own origin, so the caller
//! positions them inside a `relative()` container. Nothing here touches the
//! store or a window, which is what lets the geometry be tested on its own.
//!
//! Not here: drawing a divider (`divider`) or what a drag does to the ratios
//! (`workspace_window::layout`).

use gpui_kit::Bounds;
use gpui_kit::Pixels;
use gpui_kit::Size;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;
use knot_core::LayoutMode;

use crate::consts;

#[cfg(test)]
mod tests;

/// Which of a layout's two ratios a divider moves: `r` (`Primary`, the
/// persisted `split_ratio`) or `s` (`Secondary`, `split_ratio_secondary`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::workspace_window) enum DividerId {
    Primary,
    Secondary,
}

/// The direction a divider's line runs. A `Vertical` line separates columns
/// and is dragged sideways; a `Horizontal` one separates rows and is dragged
/// up and down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::workspace_window) enum Orientation {
    Vertical,
    Horizontal,
}

/// One divider of a layout: which ratio it moves, and which way it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::workspace_window) struct Divider {
    pub(in crate::workspace_window) id:          DividerId,
    pub(in crate::workspace_window) orientation: Orientation,
}

/// A divider and the hit area it occupies, relative to the layout area. The
/// visible line is the centre of the hit area, along its length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::workspace_window) struct DividerRect {
    pub(in crate::workspace_window) divider: Divider,
    pub(in crate::workspace_window) hit:     Bounds<Pixels>,
}

/// `ratio` held to the bounds a divider may reach.
pub(in crate::workspace_window) fn clamp_ratio(ratio: f64) -> f64 {
    ratio.clamp(knot_core::consts::WORKSPACE_SPLIT_RATIO_MIN,
                knot_core::consts::WORKSPACE_SPLIT_RATIO_MAX)
}

/// The dividers `mode` has, in the order they are drawn.
pub(in crate::workspace_window) fn dividers(mode: LayoutMode) -> &'static [Divider] {
    const VERTICAL_PRIMARY: Divider = Divider { id:          DividerId::Primary,
                                                orientation: Orientation::Vertical, };
    const HORIZONTAL_PRIMARY: Divider = Divider { id:          DividerId::Primary,
                                                  orientation: Orientation::Horizontal, };
    const HORIZONTAL_SECONDARY: Divider = Divider { id:          DividerId::Secondary,
                                                    orientation: Orientation::Horizontal, };
    match mode {
        LayoutMode::Single => &[],
        LayoutMode::SplitVertical => &[VERTICAL_PRIMARY],
        LayoutMode::SplitHorizontal => &[HORIZONTAL_PRIMARY],
        LayoutMode::ThreePane | LayoutMode::GridFourPane => {
            &[VERTICAL_PRIMARY, HORIZONTAL_SECONDARY]
        }
    }
}

/// The pane rectangles for `mode`, in pane order, filling `area`.
///
/// `r` and `s` are clamped here as well as where they are written, so a
/// ratio read from a document that predates the clamp still lays out within
/// bounds. `s` only matters to the three-pane and grid layouts.
pub(in crate::workspace_window) fn pane_rects(mode: LayoutMode, r: f64, s: f64,
                                              area: Size<Pixels>)
                                              -> Vec<Bounds<Pixels>> {
    let (r, s) = (clamp_ratio(r) as f32, clamp_ratio(s) as f32);
    let (w, h) = (area.width, area.height);
    let at = |x: Pixels, y: Pixels, width: Pixels, height: Pixels| Bounds { origin: point(x, y),
                                                                            size:   size(width,
                                                                                         height), };
    let (left, top) = (w * r, h * s);
    match mode {
        LayoutMode::Single => vec![at(px(0.), px(0.), w, h)],
        LayoutMode::SplitVertical => {
            vec![at(px(0.), px(0.), left, h), at(left, px(0.), w - left, h)]
        }
        LayoutMode::SplitHorizontal => {
            let top = h * r;
            vec![at(px(0.), px(0.), w, top), at(px(0.), top, w, h - top)]
        }
        // Pane 0 is the full-height left column; the right column is split
        // at `s` into panes 1 and 2.
        LayoutMode::ThreePane => vec![at(px(0.), px(0.), left, h),
                                      at(left, px(0.), w - left, top),
                                      at(left, top, w - left, h - top)],
        // Both dividers run the full length, so the rows line up across the
        // two columns: top left, top right, bottom left, bottom right.
        LayoutMode::GridFourPane => vec![at(px(0.), px(0.), left, top),
                                         at(left, px(0.), w - left, top),
                                         at(px(0.), top, left, h - top),
                                         at(left, top, w - left, h - top)],
    }
}

/// The dividers of `mode` with their hit areas, filling `area`.
///
/// The hit area is `DIVIDER_HIT_WIDTH` thick, centred on the line, and runs
/// the divider's whole length. In three-pane the secondary divider is the
/// right column's only: it starts at the primary divider and ends at the
/// area's right edge, so it never crosses pane 0. In the grid both run the
/// full width or height.
pub(in crate::workspace_window) fn divider_rects(mode: LayoutMode, r: f64, s: f64,
                                                 area: Size<Pixels>)
                                                 -> Vec<DividerRect> {
    let (r, s) = (clamp_ratio(r) as f32, clamp_ratio(s) as f32);
    let (w, h) = (area.width, area.height);
    let thick = px(consts::DIVIDER_HIT_WIDTH);
    let half = thick / 2.;
    let column_split = w * r;
    dividers(mode).iter()
                  .map(|&divider| {
                      let hit = match (mode, divider.id, divider.orientation) {
                          (_, DividerId::Primary, Orientation::Vertical) => {
                              Bounds { origin: point(column_split - half, px(0.)),
                                       size:   size(thick, h), }
                          }
                          (_, DividerId::Primary, Orientation::Horizontal) => {
                              Bounds { origin: point(px(0.), h * r - half),
                                       size:   size(w, thick), }
                          }
                          (LayoutMode::ThreePane, DividerId::Secondary, _) => {
                              Bounds { origin: point(column_split, h * s - half),
                                       size:   size(w - column_split, thick), }
                          }
                          (_, DividerId::Secondary, _) => Bounds { origin: point(px(0.),
                                                                                 h * s - half),
                                                                   size:   size(w, thick), },
                      };
                      DividerRect { divider, hit }
                  })
                  .collect()
}

/// The ratio a drag produces: `start_ratio` moved by the pointer's travel
/// along the divider's axis, as a share of `extent`, held to the bounds.
///
/// `extent` is the layout area's width for a divider between columns and its
/// height for one between rows - the length the ratio is a share of. A zero
/// extent (an area not yet laid out) leaves the ratio where it started.
pub(in crate::workspace_window) fn dragged_ratio(start_ratio: f64, travel: Pixels,
                                                 extent: Pixels)
                                                 -> f64 {
    let extent = f64::from(f32::from(extent));
    if extent <= 0. {
        return clamp_ratio(start_ratio);
    }
    clamp_ratio(start_ratio + f64::from(f32::from(travel)) / extent)
}

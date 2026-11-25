//! Multi-pane layout geometry: how big a divider's hit area and line are.

// The ratio bounds and default are
// `knot_core::consts::WORKSPACE_SPLIT_RATIO_*`, shared with the store's
// normalization so the two cannot disagree.

/// A divider's grabbable width, centred on its line. Wider than the line so
/// it can be hit without pixel hunting.
pub(crate) const DIVIDER_HIT_WIDTH: f32 = 12.;

/// The visible line at the centre of a divider's hit area.
pub(crate) const DIVIDER_LINE_WIDTH: f32 = 1.;

/// The opacity of the black overlay that dims every pane but the focused one
/// (`workspace-layouts`, "The focused pane"). The Swift app's value
/// (`Theme.swift`:15).
pub(crate) const UNFOCUSED_PANE_OVERLAY_OPACITY: f32 = 0.1;

//! Multi-pane layout geometry: how far a divider may be dragged and how big
//! its hit area is.

/// The bounds a split ratio is clamped to, as a share of the layout area
/// (`workspace-layouts`, "Dividers"). Either side of a divider keeps at least
/// a quarter of the area.
///
/// NOTE: `knot-agents` is adding the same bounds for the store's own
/// normalization (task 2). The two are meant to be unified when the branches
/// merge; this pair serves the window's geometry and drag.
pub(crate) const PANE_SPLIT_RATIO_MIN: f64 = 0.25;
pub(crate) const PANE_SPLIT_RATIO_MAX: f64 = 0.75;

/// The secondary ratio of a three-pane layout whose document carries none.
pub(crate) const PANE_SPLIT_RATIO_DEFAULT: f64 = 0.5;

/// A divider's grabbable width, centred on its line. Wider than the line so
/// it can be hit without pixel hunting.
pub(crate) const DIVIDER_HIT_WIDTH: f32 = 12.;

/// The visible line at the centre of a divider's hit area.
pub(crate) const DIVIDER_LINE_WIDTH: f32 = 1.;

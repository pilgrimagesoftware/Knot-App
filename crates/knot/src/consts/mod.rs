//! Crate-wide constants for the `knot` binary.
//!
//! Every other crate in the workspace keeps its tunables in one `consts.rs`;
//! this one had none, so poll intervals, cache lifetimes and palette values
//! were literals scattered through the views that happened to use them -
//! which is how the same green ended up written three ways and two different
//! repaint cadences ended up describing the same spinner.
//!
//! A value belongs here when it is a *decision* (how often to poll, how
//! stale a cache may get, what colour a state is). A value stays inline when
//! it is part of one element's layout - a padding, a gap, a single width.

mod agents;
mod bug_report;
mod cadence;
mod integrations;
mod palette;
mod pane_layout;
mod panel_geometry;
mod scheduling;
mod search;
mod windows;

pub(crate) use agents::*;
pub(crate) use bug_report::*;
pub(crate) use cadence::*;
pub(crate) use integrations::*;
pub(crate) use palette::*;
pub(crate) use pane_layout::*;
pub(crate) use panel_geometry::*;
pub(crate) use scheduling::*;
pub(crate) use search::*;
pub(crate) use windows::*;

//! When an agent starts on its own, and when it stops for idling (#404).
//!
//! Contract: `openspec/changes/session-scheduling` (the `session-scheduling`
//! capability). This module owns the vocabulary and the decisions, all pure:
//! what a schedule is and how it is stored ([`schedule`]), when it next comes
//! due ([`evaluate`]), whether a due run starts, is skipped or was missed
//! ([`decision`]), and the idle policy ([`idle`]). It does not own timers,
//! sessions or windows; the app reads these answers and acts on them.

mod decision;
mod definition;
mod evaluate;
mod idle;

pub use decision::{GRACE, RunDecision, SkipReason, decide};
pub use definition::{Preset, Schedule, TimeOfDay, Weekday, WeekdaySet};
pub use evaluate::next_run;
pub use idle::{IdleAction, IdlePolicy, IdleRule, lenient_idle_rule};

#[cfg(test)]
mod tests;

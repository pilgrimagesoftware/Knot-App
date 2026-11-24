//! Scheduled starts and idle shutdown: tick intervals and the countdown window.

use std::time::Duration;

/// How often the scheduled-start ticker checks every scheduled agent (#404).
/// Runs are minute-granular and the grace window is ten minutes, so thirty
/// seconds starts a run within half a minute of its time without busying the
/// main thread.
pub(crate) const SCHEDULE_TICK_INTERVAL: Duration = Duration::from_secs(30);

/// How often a workspace window checks its agents for idling (#404). Idle
/// periods are minutes long; checking every frame would only spend the main
/// thread.
pub(crate) const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(15);

/// How often background processes are sampled for idle tracking alone, when
/// no processes section is open (#404). Open, the section's own interval
/// applies.
pub(crate) const IDLE_PROCESS_SAMPLE_INTERVAL: Duration = Duration::from_secs(60);

/// How close to an idle shutdown the countdown appears: the smaller of this
/// and half the idle period (#404, `agent-list-ui`).
pub(crate) const IDLE_COUNTDOWN_WINDOW: Duration = Duration::from_secs(10 * 60);

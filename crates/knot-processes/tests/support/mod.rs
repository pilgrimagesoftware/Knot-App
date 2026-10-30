//! What the integration tests share.

use std::time::Duration;

/// The budget for each `ps` and `kill` a test runs through the crate's test
/// seams. It only stops a genuinely stuck test; nothing is decided by it
/// running out, so it is far past any spawn delay. The production
/// `DEFAULT_TIMEOUT` of 10 s raced `ps` starting on a loaded machine and
/// failed correct runs as `Timeout`.
pub const HANG_GUARD: Duration = Duration::from_secs(120);

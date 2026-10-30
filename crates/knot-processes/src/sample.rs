//! The blocking entry point: read the machine's process table once.
//!
//! Blocking, and on a busy machine not cheap -- wrap it in `spawn_blocking`.
//! One call serves every agent observed in a window; see [`crate::table`].

use std::time::Duration;

use crate::command::run_checked_with_timeout;
use crate::consts::{DEFAULT_TIMEOUT, PS_ARGS, PS_PROGRAM};
use crate::error::Result;
use crate::record::parse_table;
use crate::table::ProcessTable;

/// Runs `ps`, parses its output, and returns the whole table.
pub fn sample() -> Result<ProcessTable> {
    sample_within(DEFAULT_TIMEOUT)
}

/// [`sample`] with `ps` given `timeout` rather than [`DEFAULT_TIMEOUT`]; the
/// test seam's way in (`crate::test_seams`).
pub(crate) fn sample_within(timeout: Duration) -> Result<ProcessTable> {
    let output = run_checked_with_timeout(PS_PROGRAM, PS_ARGS, timeout)?;

    Ok(ProcessTable::from_records(parse_table(&output)))
}

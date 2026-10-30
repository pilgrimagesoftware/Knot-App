//! Reads the operating system's process table and turns it into the live
//! descendant tree of an agent's session root: record parsing, a transitive
//! tree walk, background/foreground classification, and terminating one
//! descendant with `TERM` escalating to `KILL`.
//!
//! Contract: `openspec/specs/agent-processes/spec.md`.
//!
//! Requires `ps` and `kill` on `PATH`. The six `ps` keywords used are
//! POSIX-stable and formatted identically by macOS `ps` and `procps-ng`.
//!
//! Also runs one shell command for the panel's `!` prefix ([`shell`]), which
//! shares the crate's process mechanics but not its lifetime: it streams.
//!
//! Runtime-agnostic: no async runtime anywhere. [`sample`] and [`terminate`]
//! block and want `spawn_blocking` at an async boundary; [`shell::spawn`] does
//! not block and owns the threads it needs.

pub mod command;
pub mod consts;
pub mod error;
pub mod record;
pub mod sample;
pub mod shell;
pub mod table;
pub mod terminate;

pub use error::{ProcessError, Result};
pub use record::{ProcessRecord, parse_table};
pub use sample::sample;
pub use shell::{ShellRequest, ShellRun, ShellRunState, ShellStatus, ShellStream};
pub use table::{Activity, DescendantProcess, ProcessTable};
pub use terminate::{Termination, TerminationTarget, terminate};

/// Test seams: [`sample`] and [`terminate`] with the budget for each `ps` and
/// `kill` they run chosen by the caller, rather than the production
/// `DEFAULT_TIMEOUT`.
///
/// Only for this crate's own integration tests, which enable the
/// `test-seams` feature through a dev-dependency on the crate itself. A
/// normal build does not compile this module, so it is no part of the API.
/// Without it those tests raced the production 10 s budget under a loaded
/// machine and failed a correct `ps` as `Timeout`.
#[cfg(feature = "test-seams")]
pub mod test_seams {
    use std::time::Duration;

    use crate::{ProcessTable, Result, Termination, TerminationTarget};

    /// [`crate::sample`], with `ps` given `timeout`.
    pub fn sample(timeout: Duration) -> Result<ProcessTable> {
        crate::sample::sample_within(timeout)
    }

    /// [`crate::terminate::terminate_with_grace`], with every `ps` and `kill`
    /// given `timeout`.
    pub fn terminate_with_grace(target: &TerminationTarget, grace: Duration, timeout: Duration)
                                -> Result<Termination> {
        crate::terminate::terminate_within(target, grace, timeout)
    }
}

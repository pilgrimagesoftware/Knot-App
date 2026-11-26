//! Whether git may run in a repository whose own configuration names
//! programs for it to run.
//!
//! A repository's `.git/config` (and files it includes) can name programs
//! git runs during ordinary commands: a filter driver's `clean` runs on
//! `status` and `add`, its `smudge` on `worktree add`, a `textconv` on a
//! diff, and `hook.<name>.command` runs even with `core.hooksPath` set to
//! `/dev/null`. No single option turns them all off. So before any git call,
//! the repository's configuration is checked, and if it names a program the
//! application's [`TrustPolicy`] decides whether the call runs.
//!
//! Contract: `openspec/specs/git-operations/spec.md`, "Repository trust".

mod detect;
mod keys;
mod policy;

pub use detect::RepositoryPrograms;
pub use keys::ProgramSetting;
pub(crate) use policy::check;
pub use policy::{TrustPolicy, configure_trust, repository_programs};

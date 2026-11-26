//! The application's answer to "may git run here", and the gate that asks
//! it before every call.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

use super::detect::{Detection, RepositoryPrograms, detect};
use crate::error::{GitError, Result};

/// Decides whether git may run in a repository whose configuration names
/// programs. Set once by the application with [`configure_trust`].
pub trait TrustPolicy: Send + Sync {
    /// Whether git may run in `programs.repository` while its configuration
    /// names `programs.settings` (never empty when asked).
    fn is_trusted(&self, programs: &RepositoryPrograms) -> bool;

    /// A call was refused because [`TrustPolicy::is_trusted`] said no. Runs
    /// on the calling thread, for every refused call, so the application
    /// dedupes before asking the user.
    fn refused(&self, programs: &RepositoryPrograms);
}

static POLICY: OnceLock<Box<dyn TrustPolicy>> = OnceLock::new();

/// Answers per folder, reused while the files they were read from are
/// unchanged. A std lock, as this crate depends on nothing else; its guard
/// never spans a git call, and a poisoned map is only stale answers.
static DETECTIONS: Mutex<BTreeMap<PathBuf, Detection>> = Mutex::new(BTreeMap::new());

/// Sets the policy every git call in this process is checked against.
/// Returns `false` if one was already set, in which case the call did
/// nothing. With none set, git runs everywhere, as it always did.
pub fn configure_trust(policy: Box<dyn TrustPolicy>) -> bool {
    POLICY.set(policy).is_ok()
}

/// The repository `folder` is in and the programs its configuration names,
/// or `None` outside a repository.
pub fn repository_programs(folder: &Path) -> Result<Option<RepositoryPrograms>> {
    let cached = DETECTIONS.lock()
                           .unwrap_or_else(PoisonError::into_inner)
                           .get(folder)
                           .filter(|detection| detection.is_current())
                           .cloned();
    if let Some(detection) = cached {
        return Ok(Some(detection.programs));
    }
    let Some(detection) = detect(folder)?
    else {
        return Ok(None);
    };
    let programs = detection.programs.clone();
    DETECTIONS.lock()
              .unwrap_or_else(PoisonError::into_inner)
              .insert(folder.to_path_buf(), detection);
    Ok(Some(programs))
}

/// Whether git may run in `folder`: always with no policy set, or when the
/// repository names no programs; otherwise as the policy says. A refusal is
/// reported to the policy and returned as [`GitError::Untrusted`].
pub(crate) fn check(folder: &Path) -> Result<()> {
    let Some(policy) = POLICY.get()
    else {
        return Ok(());
    };
    let Some(programs) = repository_programs(folder)?
    else {
        return Ok(());
    };
    if programs.settings.is_empty() || policy.is_trusted(&programs) {
        return Ok(());
    }
    policy.refused(&programs);
    Err(GitError::Untrusted { repository: programs.repository, })
}

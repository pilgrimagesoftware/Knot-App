//! The application's [`TrustPolicy`]: which repositories the user has
//! trusted to run the programs their git configuration names, and which are
//! waiting to be asked about.
//!
//! Contract: `openspec/specs/git-operations/spec.md`, "Repository trust".
//!
//! `knot-git` refuses every call in a repository that names programs and
//! reports the refusal here, from whichever thread made the call. A refusal
//! queues the repository once; the workspace window's poll sees the change
//! ([`RepositoryTrust::generation`]) and shows the first queued one. The
//! answer is the user's: trusting saves the entries the repository named at
//! that moment ([`knot_core::TrustedRepository`]), and the trust holds only
//! while the repository still names exactly those. "Not now" stops asking
//! until Knot is next started; git stays refused there meanwhile, and the
//! git panel and Settings both offer to review it again.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use knot_core::TrustedRepository;
use knot_git::{RepositoryPrograms, TrustPolicy};
use parking_lot::Mutex;

/// The registry. One per process, installed before anything runs git.
#[derive(Debug, Default)]
pub(crate) struct RepositoryTrust {
    trusted:    Mutex<Vec<TrustedRepository>>,
    /// Refused and not yet answered, in the order they were refused.
    pending:    Mutex<Vec<RepositoryPrograms>>,
    /// Answered "Not now" this session, with what each named when last
    /// refused, so it can be put back in the queue ([`Self::reconsider`]).
    declined:   Mutex<BTreeMap<PathBuf, RepositoryPrograms>>,
    /// Bumped whenever the queue changes. A counter rather than a clearing
    /// flag, so every open window sees the change, not just the first to
    /// look.
    generation: AtomicU64,
}

impl RepositoryTrust {
    pub(crate) fn new(trusted: Vec<TrustedRepository>) -> Self {
        Self { trusted: Mutex::new(trusted),
               ..Self::default() }
    }

    /// The repository to ask about next, if any.
    pub(crate) fn next_pending(&self) -> Option<RepositoryPrograms> {
        self.pending.lock().first().cloned()
    }

    /// A number that changes whenever the queue does; a window compares it
    /// with the last one it drew.
    pub(crate) fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Trusts `programs.repository` with exactly `programs.settings`,
    /// replacing any earlier trust for it, and returns the entry to save.
    pub(crate) fn trust(&self, programs: &RepositoryPrograms) -> TrustedRepository {
        let entry = TrustedRepository { path:     path_text(&programs.repository),
                                        settings: fingerprint(programs), };
        {
            let mut trusted = self.trusted.lock();
            trusted.retain(|existing| existing.path != entry.path);
            trusted.push(entry.clone());
        }
        self.answered(&programs.repository);
        entry
    }

    /// Stops asking about `repository` until the next launch, or until the
    /// user asks to [`Self::reconsider`] it.
    pub(crate) fn decline(&self, repository: &Path) {
        let queued = self.pending
                         .lock()
                         .iter()
                         .find(|programs| programs.repository == repository)
                         .cloned();
        if let Some(programs) = queued {
            self.declined
                .lock()
                .insert(repository.to_path_buf(), programs);
        }
        self.answered(repository);
    }

    /// The repositories answered "Not now" this session, by path.
    pub(crate) fn declined(&self) -> Vec<RepositoryPrograms> {
        self.declined.lock().values().cloned().collect()
    }

    /// Puts a repository answered "Not now" back in the queue, so the
    /// banner asks about it again. Nothing happens for one not declined.
    pub(crate) fn reconsider(&self, repository: &Path) {
        let Some(programs) = self.declined.lock().remove(repository)
        else {
            return;
        };
        let mut pending = self.pending.lock();
        if !pending.iter().any(|queued| queued.repository == repository) {
            pending.push(programs);
        }
        drop(pending);
        self.generation.fetch_add(1, Ordering::AcqRel);
    }

    /// Withdraws trust for the repository at `path`; the next git call
    /// there is refused and asked about again.
    pub(crate) fn revoke(&self, path: &str) {
        self.trusted.lock().retain(|existing| existing.path != path);
        self.declined.lock().remove(Path::new(path));
    }

    fn answered(&self, repository: &Path) {
        self.pending
            .lock()
            .retain(|programs| programs.repository != repository);
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
}

impl TrustPolicy for RepositoryTrust {
    fn is_trusted(&self, programs: &RepositoryPrograms) -> bool {
        let path = path_text(&programs.repository);
        let settings = fingerprint(programs);
        self.trusted
            .lock()
            .iter()
            .any(|entry| entry.path == path && entry.settings == settings)
    }

    fn refused(&self, programs: &RepositoryPrograms) {
        if let Some(declined) = self.declined.lock().get_mut(&programs.repository) {
            // Not asked again, but kept current for a later reconsider.
            declined.clone_from(programs);
            return;
        }
        let mut pending = self.pending.lock();
        match pending.iter_mut()
                     .find(|queued| queued.repository == programs.repository)
        {
            // Asked about already: keep what it names current.
            Some(queued) if queued == programs => return,
            Some(queued) => *queued = programs.clone(),
            None => pending.push(programs.clone()),
        }
        drop(pending);
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
}

/// The entries trust is given for, as saved: `key=value`, unescaped.
fn fingerprint(programs: &RepositoryPrograms) -> Vec<String> {
    programs.settings
            .iter()
            .map(|setting| format!("{}={}", setting.key, setting.value))
            .collect()
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Lets `knot-git`'s set-once policy share the registry the windows read.
struct Shared(Arc<RepositoryTrust>);

impl TrustPolicy for Shared {
    fn is_trusted(&self, programs: &RepositoryPrograms) -> bool {
        self.0.is_trusted(programs)
    }

    fn refused(&self, programs: &RepositoryPrograms) {
        self.0.refused(programs);
    }
}

static REGISTRY: OnceLock<Arc<RepositoryTrust>> = OnceLock::new();

/// Builds the registry from the saved trust and makes it `knot-git`'s
/// policy. Called once at startup, before anything runs git.
pub(crate) fn install(trusted: Vec<TrustedRepository>) {
    let registry = REGISTRY.get_or_init(|| Arc::new(RepositoryTrust::new(trusted)));
    knot_git::configure_trust(Box::new(Shared(Arc::clone(registry))));
}

/// The process's registry, once [`install`] has run.
pub(crate) fn registry() -> Option<&'static Arc<RepositoryTrust>> {
    REGISTRY.get()
}

#[cfg(test)]
mod tests;

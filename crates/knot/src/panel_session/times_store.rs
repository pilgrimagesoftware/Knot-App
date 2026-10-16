//! Where a session's conversation times are kept between launches (#220):
//! one document per ACP session under
//! `knot_core::consts::CONVERSATION_TIMES_DIR`, written whenever a turn
//! ends and read back before a restored session replays. What the times
//! mean, and how a replay is matched to them, is
//! `crate::panel_state::EntryTimes`'s business.
//!
//! Best-effort throughout: a missing or unreadable document is a session
//! with no known times, which shows its replayed entries without one. Never
//! a reason to fail the connection.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::panel_state::TimedEntry;

/// The directory the documents live in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimesStore {
    dir: PathBuf,
}

impl TimesStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The document for `session_id`. An id is the adapter's to choose, so
    /// anything but a plain name character is replaced: an id must not be
    /// able to reach outside the directory.
    fn path(&self, session_id: &str) -> PathBuf {
        let name: String = session_id.chars()
                                     .map(|c| {
                                         if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                                             c
                                         }
                                         else {
                                             '_'
                                         }
                                     })
                                     .collect();
        self.dir.join(format!("{name}.json"))
    }

    /// `session_id`'s logged times, or none when it has no readable document.
    /// Blocking: call off the async runtime's worker threads.
    pub fn load(&self, session_id: &str) -> Vec<TimedEntry> {
        std::fs::read(self.path(session_id)).ok()
                                            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                                            .unwrap_or_default()
    }

    /// Writes `entries` as `session_id`'s document, through a temporary file
    /// so a reader never sees half of one. Blocking, like [`Self::load`].
    pub fn save(&self, session_id: &str, entries: &[TimedEntry]) -> std::io::Result<()> {
        // Unique per write: two writes sharing one temporary file could
        // rename it out from under each other.
        static WRITES: AtomicU64 = AtomicU64::new(0);
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path(session_id);
        let temporary = path.with_extension(format!("{}.{}.tmp",
                                                    std::process::id(),
                                                    WRITES.fetch_add(1, Ordering::Relaxed)));
        std::fs::write(&temporary, serde_json::to_vec(entries)?)?;
        std::fs::rename(temporary, path)
    }
}

/// Saves one session's times in the background, newest snapshot winning.
///
/// Each save runs on a blocking thread of its own, so two can overlap - a
/// turn's end and the session's, say - and finish in either order. Every
/// snapshot is numbered as it is taken; writes are serialized, and one older
/// than what is already written is dropped, so an earlier snapshot landing
/// late cannot undo a newer one.
pub struct TimesSaver {
    store:      TimesStore,
    session_id: String,
    /// The number given to the last snapshot handed over.
    taken:      AtomicU64,
    /// The number of the last snapshot written, held while writing.
    written:    Mutex<u64>,
}

impl TimesSaver {
    pub fn new(store: TimesStore, session_id: impl Into<String>) -> Arc<Self> {
        Arc::new(Self { store,
                        session_id: session_id.into(),
                        taken: AtomicU64::new(0),
                        written: Mutex::new(0) })
    }

    /// Saves `entries` off the async runtime's workers.
    pub fn save_in_background(self: &Arc<Self>, entries: Vec<TimedEntry>) {
        let number = self.taken.fetch_add(1, Ordering::SeqCst) + 1;
        let saver = Arc::clone(self);
        tokio::task::spawn_blocking(move || saver.write(number, &entries));
    }

    /// Writes snapshot `number` unless a newer one already has been.
    fn write(&self, number: u64, entries: &[TimedEntry]) {
        let mut written = self.written.lock();
        if *written >= number {
            return;
        }
        match self.store.save(&self.session_id, entries) {
            Ok(()) => *written = number,
            Err(error) => eprintln!("failed to save conversation times: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn saved_times_load_back() {
        let dir = TempDir::new().expect("a temporary directory");
        let store = TimesStore::new(dir.path().join("times"));
        let entries = vec![TimedEntry::Prompt { hash:  7,
                                                at_ms: 1_000, },
                           TimedEntry::Reply { at_ms: 2_000 }];
        store.save("sess-1", &entries).expect("saves");
        assert_eq!(store.load("sess-1"), entries);
    }

    #[test]
    fn a_session_with_no_document_has_no_times() {
        let dir = TempDir::new().expect("a temporary directory");
        assert!(TimesStore::new(dir.path()).load("never-saved").is_empty());
    }

    #[test]
    fn a_session_id_cannot_reach_outside_the_directory() {
        let store = TimesStore::new("/times");
        assert_eq!(store.path("../../etc/passwd"),
                   PathBuf::from("/times/______etc_passwd.json"));
    }

    /// The newer snapshot wins whichever write runs last.
    #[test]
    fn an_older_snapshot_landing_late_does_not_undo_a_newer_one() {
        let dir = TempDir::new().expect("a temporary directory");
        let store = TimesStore::new(dir.path());
        let saver = TimesSaver::new(store.clone(), "sess-1");
        let older = vec![TimedEntry::Reply { at_ms: 1 }];
        let newer = vec![TimedEntry::Reply { at_ms: 1 },
                         TimedEntry::Reply { at_ms: 2 }];
        saver.write(2, &newer);
        saver.write(1, &older);
        assert_eq!(store.load("sess-1"), newer);
    }

    /// Writes leave no temporary files behind and do not collide.
    #[test]
    fn repeated_saves_leave_only_the_document() {
        let dir = TempDir::new().expect("a temporary directory");
        let store = TimesStore::new(dir.path());
        for at_ms in 0..3 {
            store.save("sess-1", &[TimedEntry::Reply { at_ms }])
                 .expect("saves");
        }
        let names: Vec<_> =
            std::fs::read_dir(dir.path()).expect("lists")
                                         .map(|entry| entry.expect("an entry").file_name())
                                         .collect();
        assert_eq!(names, ["sess-1.json"]);
    }
}

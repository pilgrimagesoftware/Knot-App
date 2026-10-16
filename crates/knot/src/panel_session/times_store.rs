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
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path(session_id);
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec(entries)?)?;
        std::fs::rename(temporary, path)
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
}

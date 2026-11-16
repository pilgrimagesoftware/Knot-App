//! Ranking a file listing off the render path, with stale work cancelled
//! (#5, `openspec/changes/file-finder`).
//!
//! Shared by the `@` lookup and the file finder. A caller asks for a filter
//! with [`FileSearch::request`]; the ranking runs on a blocking task, in
//! batches, and gives up between batches once a newer request has replaced
//! it. A finished ranking lands in a slot that [`FileSearch::drain`] moves
//! into view - the window's repaint poll calls it, because the task has no
//! GPUI context to repaint from. A render reads [`FileSearch::matches_for`]
//! and never ranks.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use parking_lot::Mutex;

use crate::consts::{FILE_SEARCH_BATCH, FUZZY_MAX_RESULTS};
use crate::panel_commands::{LookupEntry, LookupMatch, fuzzy};

/// A finished ranking.
#[derive(Debug, Clone)]
struct Landed {
    filter:     String,
    generation: u64,
    matches:    Vec<LookupMatch>,
}

/// One caller's search over one listing.
#[derive(Debug, Default)]
pub(crate) struct FileSearch {
    /// Bumped by every request; a task whose number is no longer current
    /// stops at its next batch and lands nothing.
    generation: Arc<AtomicU64>,
    /// The filter last asked for, so asking again from every frame costs
    /// nothing.
    requested:  Option<String>,
    landed:     Arc<Mutex<Option<Landed>>>,
    shown:      Option<Landed>,
    task:       Option<tokio::task::JoinHandle<()>>,
}

impl FileSearch {
    /// Ranks `entries` for `filter` after `delay`, replacing any search
    /// still running. Asking for the filter already asked for does nothing.
    pub(crate) fn request(&mut self, runtime: &tokio::runtime::Handle,
                          entries: &Arc<Vec<LookupEntry>>, filter: &str, delay: Duration) {
        if self.requested.as_deref() == Some(filter) {
            return;
        }
        self.requested = Some(filter.to_owned());
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(task) = self.task.take() {
            task.abort();
        }
        let current = Arc::clone(&self.generation);
        let landed = Arc::clone(&self.landed);
        let entries = Arc::clone(entries);
        let filter = filter.to_owned();
        self.task = Some(runtime.spawn(async move {
                                    if !delay.is_zero() {
                                        tokio::time::sleep(delay).await;
                                    }
                                    let still = Arc::clone(&current);
                                    let ranked =
                                        tokio::task::spawn_blocking(move || {
                                            rank_in_batches(&entries, &filter, &still, generation)
                                 .map(|matches| (filter, matches))
                                        }).await;
                                    if let Ok(Some((filter, matches))) = ranked
                                       && current.load(Ordering::SeqCst) == generation
                                    {
                                        *landed.lock() = Some(Landed { filter,
                                                                       generation,
                                                                       matches });
                                    }
                                }));
    }

    /// Moves a landed ranking into view, saying whether the view changed.
    /// A ranking a newer request has superseded is dropped.
    pub(crate) fn drain(&mut self) -> bool {
        let Some(landed) = self.landed.lock().take()
        else {
            return false;
        };
        if landed.generation != self.generation.load(Ordering::SeqCst) {
            return false;
        }
        self.shown = Some(landed);
        true
    }

    /// The ranking for exactly `filter`, once it has landed. `None` while
    /// it is being ranked - an earlier filter's ranking is never handed
    /// out, since its rows could be chosen by mistake.
    pub(crate) fn matches_for(&self, filter: &str) -> Option<&[LookupMatch]> {
        self.shown
            .as_ref()
            .filter(|shown| shown.filter == filter)
            .map(|shown| shown.matches.as_slice())
    }

    /// Forgets what was asked and shown, for a listing that changed: the
    /// next request ranks again rather than reusing a stale answer.
    pub(crate) fn invalidate(&mut self) {
        self.requested = None;
        self.shown = None;
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

impl Drop for FileSearch {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

/// `entries` ranked for `filter`, or `None` once `current` has moved past
/// `generation`. Ranked a batch at a time and merged; the order is total,
/// so merging the batches' top results gives the whole listing's.
fn rank_in_batches(entries: &[LookupEntry], filter: &str, current: &AtomicU64, generation: u64)
                   -> Option<Vec<LookupMatch>> {
    let mut best: Vec<(i32, LookupMatch)> = Vec::new();
    for batch in entries.chunks(FILE_SEARCH_BATCH) {
        if current.load(Ordering::SeqCst) != generation {
            return None;
        }
        best.extend(fuzzy::ranked(batch, filter));
        best.sort_by(fuzzy::order);
        best.truncate(FUZZY_MAX_RESULTS);
    }
    Some(best.into_iter().map(|(_, found)| found).collect())
}

#[cfg(test)]
mod tests;

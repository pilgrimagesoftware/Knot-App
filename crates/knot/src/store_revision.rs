//! Noticing agent-store changes that no notify reached (#17).
//!
//! `set-status` and the hooks write the shared `AgentStore` from threads with
//! no GPUI context, so a window learns of the write by comparing the store's
//! `revision` with the one it last drew. Each window keeps its own copy, which
//! is what lets every window over the one store see a change.

/// Whether `current` is past the revision `drawn` last recorded, moving
/// `drawn` up to it. Compared rather than cleared, so another window reading
/// the same store has its own `drawn` and sees the change too.
pub(crate) fn revised(drawn: &mut u64, current: u64) -> bool {
    let moved = *drawn != current;
    *drawn = current;
    moved
}

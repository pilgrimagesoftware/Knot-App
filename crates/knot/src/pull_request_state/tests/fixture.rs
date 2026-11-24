//! Shared builders for the pull request state tests: a known lookup, a merged
//! one and a list of URLs.

use std::time::{Duration, SystemTime};

use knot_forge::{CheckRollup, Mergeability, PullRequestState, PullRequestStatus};

use crate::pull_request_state::PullRequestLookup;

pub(super) const ALWAYS: Duration = Duration::ZERO;
pub(super) const URL: &str = "https://github.com/acme/widget/pull/42";

pub(super) fn state(status: PullRequestStatus) -> PullRequestLookup {
    PullRequestLookup::Known(PullRequestState { number: Some(42),
                                                title: Some("Do the thing".to_string()),
                                                status,
                                                checks: Some(CheckRollup::Passing),
                                                mergeable: Mergeability::Mergeable,
                                                merged_at: None })
}

/// A merged state carrying a merge time, as a fetch of a merged pull request
/// produces. `into()` rather than a named `OffsetDateTime` so these tests need
/// no date library of their own.
pub(super) fn merged_at(when: SystemTime) -> PullRequestLookup {
    let PullRequestLookup::Known(merged) = state(PullRequestStatus::Merged)
    else {
        unreachable!("state() builds a known lookup");
    };
    PullRequestLookup::Known(PullRequestState { merged_at: Some(when.into()),
                                                ..merged })
}

pub(super) fn urls(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("https://github.com/acme/widget/pull/{n}"))
               .collect()
}

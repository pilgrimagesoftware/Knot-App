//! Tests for how the sidebar's breakdown of a workspace's pull requests is
//! counted.

use knot_forge::PullRequestStatus;

use super::fixture::{URL, state, urls};
use crate::pull_request_state::{MAX_AGE, PullRequestLookup, PullRequestStateCache, counts_for};

#[test]
fn nothing_recorded_counts_as_nothing() {
    let counts = counts_for(&PullRequestStateCache::default().snapshot(), &[]);

    assert_eq!(counts.total(), 0);
    assert!(counts.nothing_known());
}

/// The spec's scenario: four records, two open (one of them draft), one
/// merged, one closed.
#[test]
fn the_breakdown_counts_draft_as_open() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(4);
    for (url, status) in urls.iter().zip([PullRequestStatus::Open,
                                          PullRequestStatus::Draft,
                                          PullRequestStatus::Merged,
                                          PullRequestStatus::Closed])
    {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(status));
    }

    let counts = counts_for(&cache.snapshot(), &urls);

    assert_eq!(counts.open, 2, "draft counts as open");
    assert_eq!(counts.merged, 1);
    assert_eq!(counts.closed, 1);
    assert_eq!(counts.pending, 0);
    assert_eq!(counts.total(), 4);
    assert!(!counts.nothing_known());
}

/// Before the view has been shown, no state has been fetched - so the row
/// shows the total rather than a breakdown that would read as "all of these
/// are pending".
#[test]
fn with_no_state_fetched_nothing_is_known_and_the_total_still_counts() {
    let cache = PullRequestStateCache::default();
    let urls = urls(3);

    let counts = counts_for(&cache.snapshot(), &urls);

    assert!(counts.nothing_known());
    assert_eq!(counts.pending, 3);
    assert_eq!(counts.total(), 3);
}

/// The spec's scenario: three have state and one request failed. The failed
/// one is counted as pending rather than folded into a state.
#[test]
fn a_failed_fetch_is_counted_as_pending_not_as_a_state() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(4);
    for (url, status) in urls.iter().zip([PullRequestStatus::Open,
                                          PullRequestStatus::Merged,
                                          PullRequestStatus::Closed])
    {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(status));
    }
    cache.claim_refresh(urls[3].clone(), MAX_AGE)
         .expect("claimed")
         .record(PullRequestLookup::Failed);

    let counts = counts_for(&cache.snapshot(), &urls);

    assert_eq!((counts.open, counts.merged, counts.closed), (1, 1, 1));
    assert_eq!(counts.pending, 1);
    assert!(!counts.nothing_known());
}

/// The forge said one of four does not exist: counted on its own, and neither
/// as a state nor as pending.
#[test]
fn a_pull_request_that_does_not_exist_is_counted_apart() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(4);
    for url in &urls[..3] {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(PullRequestStatus::Open));
    }
    cache.claim_refresh(urls[3].clone(), MAX_AGE)
         .expect("claimed")
         .record(PullRequestLookup::NotFound);

    let counts = counts_for(&cache.snapshot(), &urls);

    assert_eq!((counts.open, counts.not_found, counts.pending), (3, 1, 0));
    assert_eq!(counts.total(), 4);
}

/// Not found is an answer: a workspace whose only records are missing pull
/// requests has a breakdown to show, not just a total.
#[test]
fn not_found_alone_counts_as_known() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(PullRequestLookup::NotFound);

    let counts = counts_for(&cache.snapshot(), &[URL.to_string()]);

    assert!(!counts.nothing_known());
}

/// A state that belongs to another workspace's record must not reach this
/// workspace's row.
#[test]
fn only_the_urls_asked_about_are_counted() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(3);
    for url in &urls {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(PullRequestStatus::Open));
    }

    let counts = counts_for(&cache.snapshot(), &urls[..1]);

    assert_eq!(counts.open, 1);
    assert_eq!(counts.total(), 1);
}

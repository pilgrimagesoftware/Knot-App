//! Tests for what the cache records and answers, how forge results map to
//! lookups, and the refresh-now one-shot.

use std::time::Duration;

use knot_forge::{ForgeError, PullRequestStatus};

use super::fixture::{ALWAYS, URL, state};
use crate::pull_request_state::{
    MAX_AGE, PullRequestLookup, PullRequestStateCache, claim_refreshes,
};

#[test]
fn a_url_with_no_answer_yet_reads_as_absent() {
    let cache = PullRequestStateCache::default();

    assert_eq!(cache.get(&URL.to_string()), None);
}

#[test]
fn a_fetched_state_is_readable_and_is_not_asked_for_again() {
    let mut cache = PullRequestStateCache::default();

    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));

    assert_eq!(cache.get(&URL.to_string()),
               Some(state(PullRequestStatus::Open)));
    assert!(cache.claim_refresh(URL.to_string(), MAX_AGE).is_none(),
            "a state within MAX_AGE must not spawn a second `gh`");
}

/// The distinction the row depends on: absent is "not asked yet", a recorded
/// `Failed` is "asked, and the fetch failed" - a finished answer, not a
/// missing one.
#[test]
fn a_failed_fetch_records_an_answer_rather_than_staying_pending() {
    let mut cache = PullRequestStateCache::default();

    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(PullRequestLookup::Failed);

    assert_eq!(cache.get(&URL.to_string()), Some(PullRequestLookup::Failed));
}

/// The spec's "refreshing does not blank the row": the value is replaced
/// when the answer lands, never cleared when the fetch starts.
#[test]
fn a_row_keeps_its_state_while_a_refresh_is_in_flight() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));

    let in_flight = cache.claim_refresh(URL.to_string(), ALWAYS)
                         .expect("claimed");

    assert_eq!(cache.get(&URL.to_string()),
               Some(state(PullRequestStatus::Open)),
               "still showing the state it has");
    in_flight.record(state(PullRequestStatus::Merged));
    assert_eq!(cache.get(&URL.to_string()),
               Some(state(PullRequestStatus::Merged)));
}

/// A pull request nobody has touched comes back the same every minute.
/// Repainting for those would undo the point of the cache.
#[test]
fn an_unchanged_state_does_not_ask_for_a_repaint() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));
    assert!(cache.take_changed());

    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));
    assert!(!cache.take_changed());

    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Merged));
    assert!(cache.take_changed(), "a merge must reach the view");
}

/// The forge's answer, sorted into the three the row distinguishes.
#[test]
fn a_forge_result_becomes_the_matching_lookup() {
    let known = state(PullRequestStatus::Open);
    let PullRequestLookup::Known(open) = known.clone()
    else {
        unreachable!("state() builds a known lookup");
    };

    assert_eq!(PullRequestLookup::from(Ok(open)), known);
    assert_eq!(PullRequestLookup::from(Err(ForgeError::NotFound("gone".to_string()))),
               PullRequestLookup::NotFound);
    assert_eq!(PullRequestLookup::from(Err(ForgeError::Timeout { command: "pr view".to_string(), })),
               PullRequestLookup::Failed);
    assert_eq!(PullRequestLookup::from(Err(ForgeError::Parse("bad".to_string()))),
               PullRequestLookup::Failed);
}

/// Only not found stops the refreshes. A state can change and a failure can
/// clear, so both are asked about again.
#[test]
fn only_not_found_is_final() {
    assert!(PullRequestLookup::NotFound.is_final());
    assert!(!PullRequestLookup::Failed.is_final());
    assert!(!state(PullRequestStatus::Open).is_final());
    assert!(!state(PullRequestStatus::Merged).is_final());
}

/// What the refresh loop asks before claiming: a final answer is held, so no
/// `gh` is spawned for it however long the window stays open.
#[test]
fn a_not_found_answer_is_held_as_final_by_the_cache() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(PullRequestLookup::NotFound);

    assert!(cache.holds(&URL.to_string(), PullRequestLookup::is_final));
}

/// Refresh now asks about a not-found pull request once more, and only once:
/// the cycle after goes back to treating the answer as final.
#[test]
fn refresh_now_asks_a_not_found_url_once_more() {
    let url = "https://github.com/acme/widget/pull/9".to_string();
    let urls = vec![url.clone()];
    let mut cache = PullRequestStateCache::default();
    let mut asked_again = std::collections::BTreeSet::new();
    let claimed = claim_refreshes(&mut cache, &urls, &mut asked_again, Duration::ZERO);
    let (_, writer) = claimed.into_iter().next().expect("first claim");
    writer.record(PullRequestLookup::NotFound);

    assert!(claim_refreshes(&mut cache, &urls, &mut asked_again, Duration::ZERO).is_empty(),
            "not found is final");

    cache.mark_all_stale();
    asked_again.insert(url.clone());
    let again = claim_refreshes(&mut cache, &urls, &mut asked_again, Duration::ZERO);

    assert_eq!(again.len(), 1, "Refresh now asks again");
    assert!(asked_again.is_empty(), "and consumes the one-shot");
    assert!(claim_refreshes(&mut cache, &urls, &mut asked_again, Duration::ZERO).is_empty(),
            "the cycle after skips it again");
}

/// A URL the one-shot names but whose claim is still fresh stays named, so a
/// fetch already in flight does not swallow the user's refresh.
#[test]
fn an_unclaimed_url_keeps_its_refresh_now() {
    let url = "https://github.com/acme/widget/pull/9".to_string();
    let urls = vec![url.clone()];
    let mut cache = PullRequestStateCache::default();
    let mut asked_again = std::collections::BTreeSet::new();
    let _in_flight = claim_refreshes(&mut cache, &urls, &mut asked_again, MAX_AGE);
    asked_again.insert(url.clone());

    assert!(claim_refreshes(&mut cache, &urls, &mut asked_again, MAX_AGE).is_empty());
    assert!(asked_again.contains(&url));
}

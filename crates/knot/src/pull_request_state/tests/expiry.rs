//! Tests for when a merged record expires from the list.

use std::time::{Duration, SystemTime};

use knot_forge::PullRequestStatus;

use super::fixture::{ALWAYS, URL, merged_at, state};
use crate::pull_request_state::{MAX_AGE, PullRequestLookup, PullRequestStateCache, expired_urls};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// A cache holding one answer for `URL`.
fn cache_of(answer: PullRequestLookup) -> std::collections::BTreeMap<String, PullRequestLookup> {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(answer);
    cache.snapshot()
}

/// A pull request that does not exist has no merge time, so it never expires:
/// only the user removes it.
#[test]
fn a_record_the_forge_cannot_find_is_kept() {
    let cache = cache_of(PullRequestLookup::NotFound);

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty());
}

#[test]
fn a_record_merged_past_the_window_expires() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - DAY - Duration::from_secs(1)));

    assert_eq!(expired_urls(&cache, &[URL.to_string()], DAY, now),
               vec![URL.to_string()]);
}

#[test]
fn a_record_merged_within_the_window_is_kept() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - Duration::from_secs(60)));

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, now).is_empty());
}

/// Nothing has been asked for it yet. There is no evidence to act on, so the
/// row stays and a later refresh reconsiders it.
#[test]
fn a_record_with_no_answer_yet_is_kept() {
    let cache = PullRequestStateCache::default().snapshot();

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty());
}

/// The fetch finished and failed - a finished answer, but not one that says
/// anything about merging. Dropping on it would lose a record over a network
/// blip.
#[test]
fn a_record_whose_fetch_failed_is_kept() {
    let cache = cache_of(PullRequestLookup::Failed);

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty());
}

#[test]
fn an_unmerged_record_is_kept_however_old() {
    for status in [PullRequestStatus::Open,
                   PullRequestStatus::Draft,
                   PullRequestStatus::Closed]
    {
        let cache = cache_of(state(status));

        assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty(),
                "{status:?}");
    }
}

/// Only the URLs the caller named are considered, so one workspace's sweep
/// cannot drop another's records.
#[test]
fn a_cached_expiry_not_in_the_named_urls_is_not_returned() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - DAY - Duration::from_secs(1)));

    let other = "https://github.com/acme/widget/pull/99".to_string();
    assert!(expired_urls(&cache, &[other], DAY, now).is_empty());
}

/// The common frame: nothing expired, so the caller does not write a file.
#[test]
fn nothing_expires_when_every_record_is_fresh() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - Duration::from_secs(1)));

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, now).is_empty());
}

/// Why the repaint chain needs an entry of its own for expiry.
///
/// A merged pull request's answer stops changing once it has merged, and
/// `record` flags the cache as changed only when the value differs. So the
/// 60-second re-fetches that keep happening while the view is open schedule no
/// frame, and expiry - which runs on the render path - would have nothing to
/// run in. `WorkspaceWindow::pull_requests_expiring` is what covers that; this
/// pins the premise it rests on.
#[test]
fn re_recording_an_unchanged_merged_state_does_not_flag_the_cache() {
    let merged = merged_at(SystemTime::now() - DAY * 2);
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(merged.clone());
    assert!(cache.take_changed(), "the first answer is a change");

    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(merged);

    assert!(!cache.take_changed(),
            "an identical answer schedules no frame - so expiry needs its own chain entry");
}

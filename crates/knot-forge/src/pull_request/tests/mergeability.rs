//! Mergeability: reading `mergeable` and `mergeStateStatus`, ranking the
//! reasons a pull request cannot merge, and asking once more when GitHub has
//! not computed the answer yet.

use super::*;

/// The captured open pull request is mergeable, so its row earns a colour
/// from one fetch.
#[test]
fn a_captured_mergeable_pull_request_reads_as_mergeable() {
    let state = parse_pull_request_state(CAPTURED_OPEN).unwrap();

    assert_eq!(state.mergeable, Mergeability::Mergeable);
}

/// The question does not arise once it is merged or closed, and GitHub
/// answers `UNKNOWN` there anyway.
#[test]
fn a_pull_request_that_is_not_open_has_no_mergeability() {
    assert_eq!(parse_pull_request_state(CAPTURED_MERGED).unwrap().mergeable,
               Mergeability::Unknown);

    let closed = r#"{"state":"CLOSED","mergeable":"MERGEABLE"}"#;
    assert_eq!(parse_pull_request_state(closed).unwrap().mergeable,
               Mergeability::Unknown,
               "a closed pull request cannot be merged whatever the field says");
}

#[test]
fn a_conflicting_pull_request_reads_as_conflicting() {
    for json in [r#"{"state":"OPEN","isDraft":false,"mergeable":"CONFLICTING"}"#,
                 r#"{"state":"OPEN","isDraft":false,"mergeStateStatus":"DIRTY"}"#]
    {
        assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
                   Mergeability::Conflicting,
                   "for {json}");
    }
}

/// A conflict is the first thing to fix, so it outranks everything else
/// wrong with the pull request - here a draft with red CI.
#[test]
fn a_conflict_outranks_every_other_reason() {
    let json = r#"{"state":"OPEN","isDraft":true,"mergeable":"CONFLICTING",
                   "mergeStateStatus":"DIRTY",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"FAILURE"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Conflicting);
}

/// `mergeable` says only that there is no conflict; `mergeStateStatus` is
/// the one field that says the branch needs updating.
#[test]
fn a_branch_behind_its_base_reads_as_behind() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BEHIND"}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Behind);
}

/// Updating the branch reruns the checks, so behind is what the user acts
/// on first.
#[test]
fn behind_outranks_running_checks() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BEHIND",
                   "statusCheckRollup":[{"status":"IN_PROGRESS"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Behind);
}

/// GitHub reports `BLOCKED` while required checks are still pending. That is
/// a wait, not a problem, so it reads as checks running.
#[test]
fn running_checks_read_as_running_even_when_github_says_blocked() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BLOCKED",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"SUCCESS"},
                                        {"status":"QUEUED"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::ChecksRunning);
}

/// Green checks and no conflict, but a required review is outstanding.
#[test]
fn a_pull_request_github_calls_blocked_is_blocked_once_checks_finish() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BLOCKED",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"SUCCESS"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Blocked);
}

/// A `gh` too old to send `mergeStateStatus` still colours a mergeable row.
#[test]
fn a_gh_without_merge_state_falls_back_to_mergeable() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE"}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Mergeable);
}

/// GitHub refuses to merge a draft, which is the whole point of marking one.
#[test]
fn a_draft_is_blocked_even_when_github_calls_it_mergeable() {
    let json = r#"{"state":"OPEN","isDraft":true,"mergeable":"MERGEABLE"}"#;
    let state = parse_pull_request_state(json).unwrap();

    assert_eq!(state.status, PullRequestStatus::Draft);
    assert_eq!(state.mergeable, Mergeability::Blocked);
}

/// A pull request with red CI is not one anybody is about to merge, whatever
/// the API says about conflicts.
#[test]
fn failing_checks_block_an_otherwise_mergeable_pull_request() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"FAILURE"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Blocked);
}

/// A row must not claim a colour it has not earned.
#[test]
fn an_uncomputed_mergeability_stays_unknown() {
    for value in [r#""UNKNOWN""#, "null"] {
        let json = format!(r#"{{"state":"OPEN","isDraft":false,"mergeable":{value}}}"#);

        assert_eq!(parse_pull_request_state(&json).unwrap().mergeable,
                   Mergeability::Unknown,
                   "for {value}");
    }
}

/// GitHub computes mergeability lazily, so the first ask for an open pull
/// request often answers "unknown" and starts the work. One retry turns that
/// into a single visible fetch.
#[test]
fn an_open_pull_request_with_unknown_mergeability_is_asked_twice() {
    let runner = StubRunner::ok(r#"{"state":"OPEN","isDraft":false,"mergeable":"UNKNOWN"}"#);

    let state = pull_request_state_with(&runner, "https://github.com/a/b/pull/1").unwrap();

    assert_eq!(runner.calls().len(), 2);
    assert_eq!(state.mergeable,
               Mergeability::Unknown,
               "still unknown, and still listed");
}

/// Nothing is left to compute once it is merged, so a second ask would be
/// one `gh` run per row for no answer.
#[test]
fn a_merged_pull_request_is_never_asked_twice() {
    let runner = StubRunner::ok(CAPTURED_MERGED);

    pull_request_state_with(&runner, "https://github.com/a/b/pull/1").unwrap();

    assert_eq!(runner.calls().len(), 1);
}

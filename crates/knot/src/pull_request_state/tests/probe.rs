//! Tests for the forge probe: when it is asked, claimed, repainted and
//! re-asked.

use std::time::Duration;

use knot_forge::ForgeAvailability;

use crate::pull_request_state::{ForgeStatus, MAX_AGE};

#[test]
fn availability_starts_unprobed() {
    let status = ForgeStatus::default();

    assert!(!status.is_ready());
    assert!(status.availability().is_none());
    assert!(!status.take_changed(), "nothing has landed to draw");
}

#[test]
fn only_a_ready_forge_is_worth_fetching_from() {
    for (availability, ready) in [(ForgeAvailability::Ready, true),
                                  (ForgeAvailability::Missing, false),
                                  (ForgeAvailability::Unauthenticated, false),
                                  (ForgeAvailability::Failed("boom".to_string()), false)]
    {
        let mut status = ForgeStatus::default();
        status.claim_probe(MAX_AGE)
              .expect("the first probe is always claimable")
              .record(availability.clone());

        assert_eq!(status.is_ready(), ready, "for {availability:?}");
        assert_eq!(status.availability(), Some(availability));
    }
}

/// The defect this shape exists for: the probe is `gh auth status`, and the
/// only caller is a render.
#[test]
fn a_probe_is_claimed_once_and_not_again_until_it_ages_out() {
    let mut status = ForgeStatus::default();

    let first = status.claim_probe(MAX_AGE);
    assert!(first.is_some(), "the first frame claims it");
    assert!(status.claim_probe(MAX_AGE).is_none(),
            "every later frame must not, or an open view spawns a subprocess per frame");

    first.expect("claimed").record(ForgeAvailability::Ready);
    assert!(status.claim_probe(MAX_AGE).is_none(),
            "and a landed answer does not re-open the claim within its age");
    assert!(status.claim_probe(Duration::ZERO).is_some(),
            "only ageing out does - which is how signing in takes effect");
}

/// Without this the answer lands in a background task and nothing redraws:
/// on a workspace with nothing else running, the view keeps whatever it had.
#[test]
fn a_landed_probe_marks_the_view_for_repaint() {
    let mut status = ForgeStatus::default();
    let writer = status.claim_probe(MAX_AGE).expect("claimed");

    assert!(!status.take_changed(), "nothing has landed yet");

    writer.record(ForgeAvailability::Unauthenticated);

    assert!(status.take_changed(), "the answer has to reach a frame");
    assert!(!status.take_changed(),
            "and the flag clears, so one answer is one repaint");
}

/// An unauthenticated forge is not a permanent verdict: the user goes and
/// runs `gh auth login`, and the open view has to notice.
#[test]
fn a_later_probe_replaces_an_earlier_answer() {
    let mut status = ForgeStatus::default();
    status.claim_probe(MAX_AGE)
          .expect("claimed")
          .record(ForgeAvailability::Unauthenticated);
    assert!(!status.is_ready());

    status.claim_probe(Duration::ZERO)
          .expect("aged out")
          .record(ForgeAvailability::Ready);

    assert!(status.is_ready(),
            "signing in has to take effect without reopening the window");
}

/// Refresh now asks `gh` again at once, and the view keeps its message until
/// the new answer lands.
#[test]
fn a_stale_probe_is_claimed_again_and_keeps_its_answer() {
    let mut status = ForgeStatus::default();
    status.claim_probe(MAX_AGE)
          .expect("claimed")
          .record(ForgeAvailability::Unauthenticated);
    assert!(status.claim_probe(MAX_AGE).is_none());

    status.mark_stale();

    assert_eq!(status.availability(),
               Some(ForgeAvailability::Unauthenticated));
    assert!(status.claim_probe(MAX_AGE).is_some());
}

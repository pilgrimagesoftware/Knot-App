//! The working turn's elapsed time and estimated tokens (#35).

use std::time::{Duration, SystemTime};

use knot_acp::{SessionEvent, SessionUpdate};

use super::*;

fn streamed(state: &mut PanelState, text: &str) {
    state.apply(SessionEvent::Update(SessionUpdate::TextDelta { text: text.to_owned(), }));
}

#[test]
fn a_running_turn_reports_its_time_and_what_it_has_written() {
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    let started = state.turn_started_at.expect("the prompt starts the turn");
    streamed(&mut state, &"x".repeat(4_000));
    streamed(&mut state, &"y".repeat(801));

    let progress = state.turn_progress(started + Duration::from_secs(42))
                        .expect("a turn is running");

    assert_eq!(progress.elapsed, Duration::from_secs(42));
    assert_eq!(progress.approx_tokens, 1_201,
               "4,801 characters, rounded up");
}

/// A new turn starts counting afresh; no turn, no progress.
#[test]
fn the_count_is_per_turn_and_ends_with_it() {
    let mut state = PanelState::new();
    assert_eq!(state.turn_progress(SystemTime::now()), None);

    state.push_user_message("first".to_string());
    streamed(&mut state, &"x".repeat(400));
    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason: "end_turn".into(), }));
    assert_eq!(state.turn_progress(SystemTime::now()),
               None,
               "the turn has ended");

    state.push_user_message("second".to_string());
    let started = state.turn_started_at.expect("a new turn");
    assert_eq!(state.turn_progress(started)
                    .map(|progress| progress.approx_tokens),
               Some(0),
               "the last turn's text is not this one's");
}

#[test]
fn the_label_reads_like_the_cli() {
    let at = |seconds, tokens| {
        TurnProgress { elapsed:       Duration::from_secs(seconds),
                       approx_tokens: tokens, }.label()
    };

    assert_eq!(at(42, 0), "42s", "nothing written yet: just the time");
    let label = at(185, 1_201);
    assert!(label.contains("3m 5s") && label.contains("1.2k") && !label.contains("%{"),
            "{label}");
    assert!(at(3_725, 34_567).contains("1h 2m"));
    assert!(at(3_725, 34_567).contains("34k"));
    assert!(at(9, 950).contains("950"));
}

/// No zero-padding, and a zero minor unit is left out (#396).
#[test]
fn the_time_drops_padding_and_zero_units() {
    let cases = [(0, "0s"),
                 (59, "59s"),
                 (60, "1m"),
                 (65, "1m 5s"),
                 (239, "3m 59s"),
                 (240, "4m"),
                 (241, "4m 1s"),
                 (3_599, "59m 59s"),
                 (3_600, "1h"),
                 (3_659, "1h"),
                 (3_725, "1h 2m"),
                 (36_000, "10h")];
    for (seconds, expected) in cases {
        assert_eq!(format_elapsed(Duration::from_secs(seconds)),
                   expected,
                   "{seconds}s");
    }
}

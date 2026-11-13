//! The session-limit scan over raw PTY output (#36).

use knot_agents::SessionLimit;

use super::SessionLimitTap;
use crate::consts::SESSION_LIMIT_MAX_LINE;

const HARD_STOP: &str = "You've hit your session limit · resets 5:20pm (America/Los_Angeles)";

fn resets(at: &str) -> Option<SessionLimit> {
    Some(SessionLimit { resets: Some(at.to_owned()), })
}

fn scan(chunks: &[&[u8]]) -> Option<SessionLimit> {
    let mut tap = SessionLimitTap::default();
    for chunk in chunks {
        tap.feed(chunk);
    }
    tap.take()
}

#[test]
fn the_hard_stop_is_read_with_its_reset_time() {
    let line = format!("{HARD_STOP}\r\n");
    assert_eq!(scan(&[line.as_bytes()]),
               resets("5:20pm (America/Los_Angeles)"));
}

#[test]
fn the_wrap_up_notice_is_a_limit_without_a_reset() {
    assert_eq!(scan(&[b"Usage limit reached \xc2\xb7 wrapping up\r\n"]),
               Some(SessionLimit { resets: None }));
}

/// Claude Code draws its notices behind a bullet or a box edge, in colour,
/// and with a typographic apostrophe.
#[test]
fn decoration_and_escapes_around_a_notice_are_skipped() {
    let line = "\x1b[2K\x1b[1G\x1b[31m  \u{23bf}  You\u{2019}ve hit your session limit \u{b7} \
                resets 9am (Europe/Paris)\x1b[39m\r\n";
    assert_eq!(scan(&[line.as_bytes()]), resets("9am (Europe/Paris)"));
}

#[test]
fn a_notice_split_across_reads_is_still_read() {
    let line = format!("{HARD_STOP}\n");
    let (head, tail) = line.as_bytes().split_at(12);
    // The escape is split too: ESC in one read, its `[31m` in the next.
    assert_eq!(scan(&[head, b"\x1b", b"[31m", tail]),
               resets("5:20pm (America/Los_Angeles)"));
}

/// A cursor move ends a line: what follows it is drawn somewhere else.
#[test]
fn a_cursor_move_starts_a_new_line() {
    assert_eq!(scan(&[b"Working...\x1b[3;1HUsage limit reached\x1b[K"]),
               Some(SessionLimit { resets: None }));
}

/// Anchored at the line's start: an agent discussing limits mid-sentence,
/// or in a long line, is not stopped by it.
#[test]
fn a_notice_not_opening_a_short_line_is_not_one() {
    assert_eq!(scan(&[b"\xe2\x97\x8f I checked: usage limit reached is matched.\n"]),
               None);
    // Opening the line, but prose: the phrase runs on into a sentence.
    assert_eq!(scan(&[b"\xe2\x97\x8f Usage limit reached is now detected in the tap.\n"]),
               None);
    let long = format!("Usage limit reached {}\n",
                       "x".repeat(SESSION_LIMIT_MAX_LINE));
    assert_eq!(scan(&[long.as_bytes()]), None);
}

/// The status area is redrawn every frame; one notice is one report.
#[test]
fn a_redrawn_notice_is_reported_once() {
    let mut tap = SessionLimitTap::default();
    let line = format!("{HARD_STOP}\r\n");

    tap.feed(line.as_bytes());
    assert!(tap.take().is_some());
    tap.feed(line.as_bytes());
    assert_eq!(tap.take(), None);
}

/// Lifted by a submitted prompt, then refused again with the same reset
/// time: that is a new limit, and is reported (#377 review).
#[test]
fn the_same_notice_after_a_lift_is_reported_again() {
    let mut tap = SessionLimitTap::default();
    let line = format!("{HARD_STOP}\r\n");

    tap.feed(line.as_bytes());
    tap.take();
    tap.rearm();
    tap.feed(line.as_bytes());
    assert_eq!(tap.take(), resets("5:20pm (America/Los_Angeles)"));
}

/// The wrap-up notice first, then the hard stop with its reset: the second
/// says more, so it replaces the first.
#[test]
fn a_different_notice_is_reported() {
    let mut tap = SessionLimitTap::default();

    tap.feed(b"Usage limit reached\n");
    tap.take();
    tap.feed(format!("{HARD_STOP}\n").as_bytes());
    assert_eq!(tap.take(), resets("5:20pm (America/Los_Angeles)"));
}

#[test]
fn an_osc_title_is_not_read_as_text() {
    assert_eq!(scan(&[b"\x1b]0;Usage limit reached\x07plain output\n"]),
               None);
}

/// #397: the hard stop as Claude Code prints it now, naming its limit, drawn
/// the way it draws the older one.
#[test]
fn the_renamed_hard_stop_is_read_with_its_reset_time() {
    for name in ["5-hour limit", "weekly limit", "Opus limit"] {
        let line = format!("\x1b[31m  \u{23bf}  You\u{2019}ve hit your {name} \u{b7} resets 9am \
                            (Europe/Paris)\x1b[39m\r\n");
        assert_eq!(scan(&[line.as_bytes()]),
                   resets("9am (Europe/Paris)"),
                   "{name}");
    }
}

/// Fast mode's cooldown uses the same lead and the agent carries on at
/// normal speed, so it is not a stop.
#[test]
fn the_fast_mode_cooldown_is_not_a_limit() {
    assert_eq!(scan(&[b"You've hit your fast limit \xc2\xb7 resets in 4m\r\n"]),
               None);
}

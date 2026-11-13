//! Watching a terminal agent's output for its usage-limit notice (#36).
//!
//! A Panel-mode agent learns of the limit from the error that answers its
//! prompt (#318). A terminal agent prints it instead - Claude Code's
//! "You've hit your 5-hour limit · resets 5:20pm (America/Los_Angeles)",
//! "session limit" before #397,
//! or "Usage limit reached · wrapping up" when it is first allowed to finish
//! what it was doing - and this is where Knot reads it.
//!
//! Like the pull request scan beside it, this reads the PTY byte stream,
//! not the rendered grid: every byte once, on the reader thread. Escape
//! sequences are dropped as they arrive, a cursor move ends a line the way a
//! newline does, and only a line's start is compared - one short prefix
//! check per line, with nothing buffered past [`SESSION_LIMIT_MAX_LINE`].

use knot_agents::SessionLimit;

use crate::consts::{SESSION_LIMIT_MAX_LINE, SESSION_LIMIT_NOTICES, SESSION_LIMIT_SEPARATORS};

/// Where the scan is inside an escape sequence, carried across chunks
/// because a PTY read can split one anywhere.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Escape {
    #[default]
    None,
    /// After ESC.
    Esc,
    /// Inside `ESC [`, until its final byte.
    Csi,
    /// Inside `ESC ]`, until BEL or `ESC \`.
    Osc,
    /// ESC inside an OSC: the start of its `ESC \` terminator.
    OscEsc,
}

/// The scan and what it has found, fed on the PTY reader thread and taken
/// by the window's poll.
#[derive(Debug, Default)]
pub(crate) struct SessionLimitTap {
    escape:   Escape,
    /// The current line, printable bytes only.
    line:     Vec<u8>,
    /// The current line outgrew [`SESSION_LIMIT_MAX_LINE`]: not a notice.
    overflow: bool,
    /// The limit last reported. The UI redraws its status area every frame,
    /// so one notice is seen many times; it is reported once, until
    /// [`Self::rearm`] says the limit it reported has been lifted.
    last:     Option<SessionLimit>,
    /// Found but not yet handed to the window.
    pending:  Option<SessionLimit>,
}

impl SessionLimitTap {
    /// Scan a chunk of PTY output.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.step(byte);
        }
        // The notice can be the last line drawn, with no newline after it
        // until the next frame.
        self.check_line();
    }

    /// Take the limit seen since the last call, if any.
    pub(crate) fn take(&mut self) -> Option<SessionLimit> {
        self.pending.take()
    }

    /// The limit last reported was lifted: report the next notice even if
    /// it reads the same - a prompt refused again after the lift carries
    /// the same "resets" as the refusal before it.
    pub(crate) fn rearm(&mut self) {
        self.last = None;
    }

    fn step(&mut self, byte: u8) {
        self.escape = match (self.escape, byte) {
            (Escape::None, 0x1B) => Escape::Esc,
            (Escape::None, b'\n' | b'\r') => {
                self.end_line();
                Escape::None
            }
            (Escape::None, byte) => {
                if byte >= 0x20 || byte == b'\t' {
                    self.push(byte);
                }
                Escape::None
            }
            (Escape::Esc, b'[') => Escape::Csi,
            (Escape::Esc, b']') => Escape::Osc,
            (Escape::Esc, _) => Escape::None,
            (Escape::Csi, 0x40..=0x7E) => {
                match byte {
                    // Cursor forward: a gap in the same line.
                    b'C' => self.push(b' '),
                    // Any other cursor move leaves the line.
                    b'A' | b'B' | b'E' | b'F' | b'G' | b'H' | b'd' | b'f' => self.end_line(),
                    _ => {}
                }
                Escape::None
            }
            (Escape::Csi, _) => Escape::Csi,
            (Escape::Osc, 0x07) => Escape::None,
            (Escape::Osc, 0x1B) => Escape::OscEsc,
            (Escape::Osc, _) => Escape::Osc,
            (Escape::OscEsc, b'\\') => Escape::None,
            (Escape::OscEsc, _) => Escape::Osc,
        };
    }

    fn push(&mut self, byte: u8) {
        if self.line.len() < SESSION_LIMIT_MAX_LINE {
            self.line.push(byte);
        }
        else {
            self.overflow = true;
        }
    }

    fn end_line(&mut self) {
        self.check_line();
        self.line.clear();
        self.overflow = false;
    }

    fn check_line(&mut self) {
        if self.overflow || self.line.is_empty() {
            return;
        }
        let Some(limit) = notice(&String::from_utf8_lossy(&self.line))
        else {
            return;
        };
        if self.last.as_ref() != Some(&limit) {
            self.last = Some(limit.clone());
            self.pending = Some(limit);
        }
    }
}

/// The limit `line` announces, if it opens with one: a named limit, or a
/// notice that is followed by nothing or a separator. Leading decoration - a
/// bullet, a box edge, indentation - is skipped first.
fn notice(line: &str) -> Option<SessionLimit> {
    let text = line.trim_start_matches(|c: char| !c.is_alphanumeric())
                   .trim_end();
    let lower = text.to_lowercase().replace('\u{2019}', "'");
    let is_notice = SessionLimit::opens_with_named_limit(&lower)
                    || SESSION_LIMIT_NOTICES.iter().any(|notice| {
                                                       lower.strip_prefix(notice)
                                                            .is_some_and(|rest| {
                                                                let rest = rest.trim_start();
                                                                rest.is_empty()
                                                       || rest.starts_with(SESSION_LIMIT_SEPARATORS)
                                                            })
                                                   });
    if !is_notice {
        return None;
    }
    Some(SessionLimit::from_error(text).unwrap_or(SessionLimit { resets: None }))
}

#[cfg(test)]
mod tests;

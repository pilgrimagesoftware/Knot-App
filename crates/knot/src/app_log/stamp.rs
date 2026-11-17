//! Puts a time on every line of the application log (#458).
//!
//! Once stderr is redirected, fd 2 is the write end of a pipe rather than the
//! log file, and one thread reads the pipe a line at a time and writes each
//! line to the file behind an RFC 3339 UTC time - the format
//! `knot-mcp.jsonl` uses, so the two files line up. Doing it at the
//! descriptor covers every writer at once: each `eprintln!`, adapter stderr
//! forwarded by `knot-acp`, a panic's message, and anything a library or a
//! child process that inherited fd 2 writes there.
//!
//! The pipe is what a crash would lose: lines still in it when the process
//! dies never reach the file. [`flush`] closes that gap where it matters,
//! after a panic's message and before an exit.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use parking_lot::Condvar;
use parking_lot::Mutex;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::consts::APP_LOG_FLUSH_TIMEOUT;

/// Written to stderr by [`flush`]. The stamping thread drops it rather than
/// logging it, and counts it: by then everything written before it is in
/// the file, since a pipe keeps order. The escape keeps it from matching
/// anything a person or an adapter would print.
const FLUSH_MARKER: &[u8] = b"\x1b[knot-log-flush]";

/// How many flush markers the stamping thread has passed, for [`flush`] to
/// wait on.
#[derive(Default)]
pub(super) struct Drained {
    seen:    Mutex<u64>,
    changed: Condvar,
}

impl Drained {
    fn mark(&self) {
        *self.seen.lock() += 1;
        self.changed.notify_all();
    }

    fn seen(&self) -> u64 {
        *self.seen.lock()
    }

    /// Waits until more than `before` markers have passed, or `timeout`.
    /// Reports whether they did.
    fn wait_past(&self, before: u64, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut seen = self.seen.lock();
        while *seen <= before {
            if self.changed.wait_until(&mut seen, deadline).timed_out() {
                return *seen > before;
            }
        }
        true
    }
}

/// Set once stderr runs through the stamping thread; [`flush`] does nothing
/// before that, or when stderr is a terminal.
static DRAINED: OnceLock<Drained> = OnceLock::new();

/// Points fd 2 at a pipe whose lines a background thread stamps and writes
/// to `log`.
#[cfg(unix)]
pub(super) fn redirect_through_stamper(log: std::fs::File) -> std::io::Result<()> {
    let (reader, writer) = std::io::pipe()?;
    let drained = DRAINED.get_or_init(Drained::default);
    std::thread::Builder::new().name("knot-log".to_string())
                               .spawn(move || copy_stamped(reader, log, drained))?;
    // On failure the writer drops here, the thread reads end-of-file and
    // exits, and stderr stays where it was.
    super::redirect_stderr(&writer)
}

#[cfg(not(unix))]
pub(super) fn redirect_through_stamper(_: std::fs::File) -> std::io::Result<()> {
    Ok(())
}

/// Copies `input` to `output` a line at a time, each behind the time it was
/// read, until every writer of `input` has closed it.
///
/// A failed write is dropped rather than retried or returned: this thread
/// stopping would leave the pipe to fill, and every `eprintln!` after that
/// would block the thread that made it - the UI's included.
pub(super) fn copy_stamped(input: impl Read, mut output: impl Write, drained: &Drained) {
    let mut reader = BufReader::new(input);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => return,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return,
        }
        let ended = line.last() == Some(&b'\n');
        if ended {
            line.pop();
        }
        let flushing = line.ends_with(FLUSH_MARKER);
        if flushing {
            // Another thread's unfinished line can sit in front of the
            // marker; it is still a line, and still logged.
            line.truncate(line.len() - FLUSH_MARKER.len());
        }
        if !flushing || !line.is_empty() {
            let _ = output.write_all(&stamped(OffsetDateTime::now_utc(), &line));
        }
        if flushing {
            drained.mark();
        }
    }
}

/// `line`, without its newline, as the log holds it: `<time> <line>\n`.
pub(super) fn stamped(at: OffsetDateTime, line: &[u8]) -> Vec<u8> {
    let time = at.format(&Rfc3339)
                 .unwrap_or_else(|_| at.unix_timestamp().to_string());
    let mut out = Vec::with_capacity(time.len() + line.len() + 2);
    out.extend_from_slice(time.as_bytes());
    out.push(b' ');
    out.extend_from_slice(line);
    out.push(b'\n');
    out
}

/// Waits, briefly, until everything written to stderr so far is in the log.
///
/// For the moments a process may end with lines still in the pipe: after a
/// panic's message, and before an exit. Returns at once when stderr is not
/// running through the stamping thread, and after
/// [`APP_LOG_FLUSH_TIMEOUT`] at the latest - a stuck log must not hold up
/// a quit.
pub(crate) fn flush() {
    let Some(drained) = DRAINED.get()
    else {
        return;
    };
    let before = drained.seen();
    let mut marker = FLUSH_MARKER.to_vec();
    marker.push(b'\n');
    // One write, so no other thread's output lands between marker and
    // newline.
    if std::io::stderr().write_all(&marker).is_err() {
        return;
    }
    drained.wait_past(before, APP_LOG_FLUSH_TIMEOUT);
}

#[cfg(test)]
mod tests;

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

#[cfg(unix)]
use std::io::BufRead;
#[cfg(unix)]
use std::io::BufReader;
#[cfg(unix)]
use std::io::Read;
use std::io::Write;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use parking_lot::Condvar;
use parking_lot::Mutex;
#[cfg(unix)]
use time::OffsetDateTime;
#[cfg(unix)]
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
    // Only the stamping thread marks, and only Unix runs one.
    #[cfg(unix)]
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

// Unix only, like `redirect_through_stamper`: elsewhere stderr is never
// redirected, so nothing reads a pipe.
#[cfg(unix)]
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

#[cfg(unix)]
/// `line`, without its newline, as the log holds it: `<time> <line>\n`,
/// with any Knot MCP credential in it replaced (see [`redact_credentials`]).
pub(super) fn stamped(at: OffsetDateTime, line: &[u8]) -> Vec<u8> {
    let line = redact_credentials(line);
    let line = line.as_ref();
    let time = at.format(&Rfc3339)
                 .unwrap_or_else(|_| at.unix_timestamp().to_string());
    let mut out = Vec::with_capacity(time.len() + line.len() + 2);
    out.extend_from_slice(time.as_bytes());
    out.push(b' ');
    out.extend_from_slice(line);
    out.push(b'\n');
    out
}

/// `line` with every Knot MCP credential (`knot_a_`, `knot_h_`, `knot_c_`)
/// replaced by `<knot-credential>`.
///
/// Knot never prints one, but this file also takes adapters' stderr and the
/// errors they return. An adapter that echoes its MCP configuration in a
/// validation error or a debug line would otherwise write the agent's
/// credential into the log on disk (security review of #521). The
/// bug-report scrubber covers what is published; this covers the file.
#[cfg(unix)]
pub(super) fn redact_credentials(line: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    static CREDENTIAL: std::sync::LazyLock<regex::bytes::Regex> = std::sync::LazyLock::new(|| {
        regex::bytes::Regex::new(r"knot_[ahc]_[A-Za-z0-9_-]{16,}")
                .expect("the credential pattern is fixed and valid")
    });
    // Most lines name no credential; skip the regex for them.
    if !line.windows(5).any(|window| window == b"knot_") {
        return std::borrow::Cow::Borrowed(line);
    }
    CREDENTIAL.replace_all(line, &b"<knot-credential>"[..])
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

// The tests drive the stamping loop through real pipes, which is Unix only.
#[cfg(all(test, unix))]
mod tests;

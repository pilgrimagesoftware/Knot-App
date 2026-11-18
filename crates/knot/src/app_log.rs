//! The application log: stderr, kept in a file when nobody is reading it.
//!
//! A packaged `Knot.app` is launched with stderr pointed at `/dev/null`, so
//! every `eprintln!` in the workspace - and the message of any panic - was
//! lost, and a bug report had no application log to attach (#515). When
//! stderr is not a terminal it is pointed at [`APP_LOG_FILE_NAME`] in the
//! platform log directory instead. Under `cargo run` it stays on the
//! terminal, where it is already read.
//!
//! Every line is written behind the UTC time it was logged at (#458), by
//! [`stamp`]: see there for how, and for what a crash can still lose.
//!
//! One file per run: the previous run's is kept as
//! [`APP_LOG_PREVIOUS_FILE_NAME`], and anything older is replaced. That bounds
//! the log without a rotating writer, which a raw file descriptor could not
//! have anyway.

use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;

use crate::consts::APP_LOG_FILE_NAME;
use crate::consts::APP_LOG_PREVIOUS_FILE_NAME;

mod stamp;

pub(crate) use stamp::flush;

/// The application log's path, whether or not this run is writing it.
pub(crate) fn app_log_path() -> Option<PathBuf> {
    knot_core::log_dir().map(|directory| directory.join(APP_LOG_FILE_NAME))
}

/// Points stderr at the application log, unless it is a terminal.
///
/// Called first thing in `run`, before anything has written to stderr. A
/// failure leaves stderr where it was: losing the log must not stop the app.
pub(crate) fn redirect_stderr_to_log() {
    if std::io::stderr().is_terminal() {
        return;
    }
    let Some(directory) = knot_core::log_dir()
    else {
        return;
    };
    match open_and_redirect(&directory) {
        Ok(()) => flush_after_panic_messages(),
        Err(error) => eprintln!("app log: stderr stays where it was: {error}"),
    }
}

fn open_and_redirect(directory: &Path) -> std::io::Result<()> {
    let file = open_fresh_log(directory)?;
    stamp::redirect_through_stamper(file)
}

/// Makes a panic wait for its own message to reach the log.
///
/// The message is written to the pipe, and a panic that ends the process
/// would otherwise take it down with whatever the stamping thread had not
/// read yet - the one line a crash report most needs (#515).
fn flush_after_panic_messages() {
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
                             report(info);
                             flush();
                         }));
}

/// Keeps the last run's log as the previous one and opens an empty current
/// one.
pub(crate) fn open_fresh_log(directory: &Path) -> std::io::Result<std::fs::File> {
    std::fs::create_dir_all(directory)?;
    let current = directory.join(APP_LOG_FILE_NAME);
    if current.exists() {
        std::fs::rename(&current, directory.join(APP_LOG_PREVIOUS_FILE_NAME))?;
    }
    std::fs::File::create(current)
}

/// Points fd 2 at `target`, the write end of the stamping pipe.
#[cfg(unix)]
fn redirect_stderr(target: &impl std::os::fd::AsRawFd) -> std::io::Result<()> {
    // SAFETY: both descriptors are open for the duration of the call -
    // `target` is borrowed, and fd 2 always exists in a process - and `dup2`
    // touches nothing but the descriptor table. Rust's stderr is unbuffered
    // and holds only the number 2, so it follows the new target without
    // being told. `target` may be dropped afterwards: fd 2 is an independent
    // duplicate.
    let result = unsafe { libc::dup2(target.as_raw_fd(), libc::STDERR_FILENO) };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests;

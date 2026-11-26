use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::consts::{DEFAULT_TIMEOUT, OUTPUT_DRAIN_FLOOR, POLL_INTERVAL};
use crate::error::{GitError, Result};
use crate::program::configured;

/// Who started a git call, which decides whether the repository's own hooks
/// run for it. Passed by the caller, never read off the arguments: the same
/// `git worktree add` is the user's from a dialog and not theirs from a tool
/// call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallOrigin {
    /// The user, from Knot's own UI (the commit window, a dialog): the
    /// repository's hooks run, as they would in a terminal.
    User,
    /// Knot on its own, or an agent through one of Knot's tools: no hooks.
    Automated,
}

/// Runs `git` in a fixed working directory with a wall-clock timeout.
///
/// Output streams drain on worker threads so a full pipe buffer never wedges
/// the child; the main thread polls for exit and kills on timeout.
///
/// The timeout bounds the whole call, output included. git can exit and
/// leave a helper it started - a credential cache daemon, an fsmonitor, an
/// ssh control master - holding the output pipes open; the streams only end
/// when the last holder closes them, so waiting on the drain threads without
/// a bound made the call last as long as that helper.
#[derive(Debug, Clone)]
pub struct Runner {
    cwd:          PathBuf,
    timeout:      Duration,
    program:      OsString,
    search_path:  Option<OsString>,
    /// Whether `program` is git, and so takes [`call_config`]'s `-c`
    /// options. Only a test runs anything else.
    is_git:       bool,
    /// Whether calls are checked against [`crate::trust`] first. Off only
    /// for the calls that do the checking.
    checks_trust: bool,
}

impl Runner {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        let configured = configured();

        Self { cwd:          cwd.into(),
               timeout:      DEFAULT_TIMEOUT,
               program:      configured.program,
               search_path:  configured.search_path,
               is_git:       true,
               checks_trust: true, }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_program(mut self, program: impl Into<OsString>) -> Self {
        self.program = program.into();
        self.is_git = false;
        self
    }

    /// This runner, for the calls that read a repository's configuration to
    /// decide whether it is trusted: they run no configured program, and
    /// checking them would recurse.
    pub(crate) fn without_trust_check(mut self) -> Self {
        self.checks_trust = false;
        self
    }

    /// This runner, timeout and program included, run from `cwd` instead.
    pub fn in_dir(&self, cwd: impl Into<PathBuf>) -> Self {
        Self { cwd: cwd.into(),
               ..self.clone() }
    }

    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    /// Runs `git <args>`. Returns stdout trimmed of surrounding whitespace.
    ///
    /// Non-zero exit -> [`GitError::Command`] carrying stderr (or stdout when
    /// stderr is empty) and the exit code. Exceeding the timeout kills the
    /// process and yields [`GitError::Timeout`].
    ///
    /// The call is [`CallOrigin::Automated`]; see [`Runner::run_as`].
    pub fn run(&self, args: &[&str]) -> Result<String> {
        self.run_as(args, CallOrigin::Automated)
    }

    /// As [`Runner::run`], for a call started by `origin`.
    pub fn run_as(&self, args: &[&str], origin: CallOrigin) -> Result<String> {
        Ok(self.run_raw_as(args, origin)?.trim().to_owned())
    }

    /// As [`Runner::run`], but returns stdout exactly as git wrote it.
    ///
    /// For output where whitespace is data rather than formatting: a
    /// NUL-separated path list whose first entry begins with a space is
    /// still that path, and trimming it produces a path that does not
    /// exist. Prefer [`Runner::run`] for everything else - most git output
    /// carries a trailing newline nobody wants.
    pub fn run_raw(&self, args: &[&str]) -> Result<String> {
        self.run_raw_as(args, CallOrigin::Automated)
    }

    fn run_raw_as(&self, args: &[&str], origin: CallOrigin) -> Result<String> {
        if self.is_git && self.checks_trust {
            crate::trust::check(&self.cwd)?;
        }
        let label = display_command(args);

        let mut command = Command::new(&self.program);
        if self.is_git {
            command.args(call_config(origin));
        }
        command.args(args).current_dir(&self.cwd);
        if let Some(search_path) = &self.search_path {
            // Git's own helpers - credential helpers, hooks, `git-lfs`,
            // `ssh` - are looked up in the environment it is handed, and a
            // GUI process's `PATH` names none of the places they install
            // into.
            command.env("PATH", search_path);
        }

        let mut child = command.stdin(Stdio::null())
                               .stdout(Stdio::piped())
                               .stderr(Stdio::piped())
                               .spawn()?;

        let mut stdout_pipe = child.stdout.take().expect("stdout piped");
        let mut stderr_pipe = child.stderr.take().expect("stderr piped");
        let stdout_reader = drain(move || read_to_string(&mut stdout_pipe));
        let stderr_reader = drain(move || read_to_string(&mut stderr_pipe));

        let deadline = Instant::now() + self.timeout;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                drop(stdout_reader);
                drop(stderr_reader);
                return Err(GitError::Timeout { command: label });
            }

            thread::sleep(POLL_INTERVAL);
        };

        let (Some(stdout), Some(stderr)) =
            (collect(&stdout_reader, deadline), collect(&stderr_reader, deadline))
        else {
            return Err(GitError::Timeout { command: label });
        };

        if status.success() {
            return Ok(stdout);
        }

        let output = if stderr.trim().is_empty() {
            stdout
        }
        else {
            stderr
        };

        Err(GitError::Command { command: label,
                                output:  output.trim().to_owned(),
                                code:    status.code().unwrap_or(-1), })
    }
}

/// Reads a stream to its end on a thread of its own, handing the text back
/// through the returned channel. The thread is detached: one still blocked
/// when [`collect`] gives up ends when the stream finally closes.
fn drain(read: impl FnOnce() -> String + Send + 'static) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(read());
    });
    rx
}

/// A drained stream's text, or `None` if it has not ended by `deadline` -
/// with at least [`OUTPUT_DRAIN_FLOOR`] allowed, so a child that exits just
/// before the deadline is not reported as timed out for want of a moment to
/// flush.
fn collect(reader: &mpsc::Receiver<String>, deadline: Instant) -> Option<String> {
    let wait = deadline.saturating_duration_since(Instant::now())
                       .max(OUTPUT_DRAIN_FLOOR);
    reader.recv_timeout(wait).ok()
}

fn read_to_string(pipe: &mut impl Read) -> String {
    let mut buf = String::new();
    let _ = pipe.read_to_string(&mut buf);
    buf
}

fn display_command(args: &[&str]) -> String {
    args.join(" ")
}

/// The `-c` options a call from `origin` runs under:
/// [`consts::CALL_CONFIG`](crate::consts::CALL_CONFIG) always, and
/// [`consts::NO_HOOKS_CONFIG`](crate::consts::NO_HOOKS_CONFIG) unless the
/// user started the call, so its hooks are theirs to run.
fn call_config(origin: CallOrigin) -> Vec<&'static str> {
    let mut config = crate::consts::CALL_CONFIG.to_vec();
    match origin {
        CallOrigin::User => {}
        CallOrigin::Automated => config.extend_from_slice(crate::consts::NO_HOOKS_CONFIG),
    }
    config
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Runner;
    use crate::error::GitError;

    /// git exits, but a helper it started keeps stdout open far past the
    /// timeout - `sh` stands in for git here. The call must end at the
    /// timeout, not when the helper does.
    #[test]
    fn a_helper_holding_the_output_does_not_outlast_the_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let runner = Runner::new(dir.path()).with_program("sh")
                                            .with_timeout(Duration::from_millis(200));

        let started = std::time::Instant::now();
        let err = runner.run(&["-c", "sleep 30 & echo started"]).unwrap_err();

        assert!(started.elapsed() < Duration::from_secs(5),
                "waited {:?} on a pipe the helper held",
                started.elapsed());
        assert!(matches!(err, GitError::Timeout { .. }), "got {err:?}");
    }

    #[test]
    fn timeout_kills_process_and_names_command() {
        let dir = tempfile::tempdir().unwrap();
        let runner = Runner::new(dir.path()).with_program("sleep")
                                            .with_timeout(Duration::from_millis(50));

        let started = std::time::Instant::now();
        let err = runner.run(&["5"]).unwrap_err();

        assert!(started.elapsed() < Duration::from_secs(2),
                "did not abort early");
        match err {
            GitError::Timeout { command } => assert_eq!(command, "5"),
            other => panic!("expected Timeout, got {other:?}"),
        }
    }
}

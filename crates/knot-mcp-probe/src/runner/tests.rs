//! These run real subprocesses, deliberately: the behaviours under test -
//! killing on timeout, closing stdin, honouring the working directory and
//! environment - are the ones a stub cannot demonstrate.
//!
//! `/bin/sh` is the program throughout, so nothing here needs an agent CLI
//! installed and the same tests run on both CI legs.

use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::{CommandRunner, McpRunner, ProbeCommand, outcome};
use crate::error::ProbeError;

fn sh(script: &str, cwd: &std::path::Path) -> ProbeCommand {
    ProbeCommand::new("/bin/sh", vec!["-c".to_owned(), script.to_owned()], cwd)
}

fn runner_with(timeout: Duration) -> CommandRunner {
    CommandRunner::new().with_timeout(timeout)
}

/// The budget for a test that is not about the timeout. It only stops a
/// genuinely stuck test; nothing is decided by it running out, so it is far
/// past any spawn delay - a fixed 10 s raced `/bin/sh` starting under load
/// and failed a correct runner. The timeout tests set their own, short one:
/// killing on time is what they test.
const HANG_GUARD: Duration = Duration::from_secs(120);

fn runner() -> CommandRunner {
    runner_with(HANG_GUARD)
}

#[test]
fn stdout_comes_back_on_success() {
    let dir = TempDir::new().unwrap();
    let out = runner().run(&sh("echo hello", dir.path()))
                      .expect("a successful command");

    assert_eq!(out.trim(), "hello");
}

/// The bound exists so a hung child cannot own the section forever. The
/// assertion that matters is not only the error but the elapsed time: a
/// timeout that did not actually kill the child would still return, late.
#[test]
fn a_hanging_command_is_killed_at_the_timeout() {
    let dir = TempDir::new().unwrap();
    let started = Instant::now();

    let err = runner_with(Duration::from_millis(200)).run(&sh("sleep 30", dir.path()))
                                                     .expect_err("a command that never exits");

    assert!(matches!(err, ProbeError::TimedOut { .. }), "got {err:?}");
    assert!(started.elapsed() < Duration::from_secs(5),
            "the child was not killed; the call took {:?}",
            started.elapsed());
}

/// A CLI that would prompt must fail rather than wait. With stdin inherited
/// `cat` reads from the test harness's own stdin and waits there, so this
/// ends as `TimedOut` at the guard; exiting at all is the proof. No clock is
/// asserted: under load, a correct run can take longer than any fixed bound.
#[test]
fn stdin_is_closed_so_a_prompting_command_does_not_wait() {
    let dir = TempDir::new().unwrap();

    let out = runner().run(&sh("cat", dir.path()))
                      .expect("cat on a closed stdin exits cleanly");

    assert_eq!(out.trim(), "");
}

/// Project-scoped MCP configuration resolves against the agent's working
/// directory. A probe run from Knot's own cwd answers a different question.
#[test]
fn the_command_runs_in_the_given_working_directory() {
    let dir = TempDir::new().unwrap();
    let expected = dir.path().file_name().unwrap().to_str().unwrap().to_owned();

    let out = runner().run(&sh("pwd", dir.path())).expect("pwd");

    assert!(out.trim().ends_with(&expected),
            "ran in {out:?}, expected a directory ending {expected:?}");
}

/// `CLAUDE_CONFIG_DIR` and friends decide which user configuration is in
/// play, so the agent's launch environment has to reach the child.
#[test]
fn the_agents_environment_reaches_the_child() {
    let dir = TempDir::new().unwrap();
    let command =
        sh("echo $KNOT_PROBE_MARKER", dir.path()).with_env(vec![("KNOT_PROBE_MARKER".to_owned(),
                                                                 "carried".to_owned())]);

    let out = runner().run(&command).expect("echo");

    assert_eq!(out.trim(), "carried");
}

#[test]
fn a_missing_program_is_reported_as_missing() {
    let dir = TempDir::new().unwrap();
    let command = ProbeCommand::new("knot-no-such-agent-cli", vec!["mcp".to_owned()], dir.path());

    let err = runner().run(&command)
                      .expect_err("a program that is not installed");

    assert!(matches!(&err, ProbeError::Missing { program } if program == "knot-no-such-agent-cli"),
            "got {err:?}");
}

/// The case that would otherwise discard the answer. `claude mcp list` exits
/// non-zero when a server fails its health check *and still lists every
/// server* - which is precisely the state the section exists to show. Judging
/// on the exit code alone would turn "one server is down" into "the probe
/// failed".
#[test]
fn a_nonzero_exit_that_still_listed_is_not_a_failure() {
    let dir = TempDir::new().unwrap();
    let out = runner().run(&sh("echo 'github: https://x - connected'; exit 1", dir.path()))
                      .expect("output survives a non-zero exit");

    assert!(out.contains("github"));
}

#[test]
fn a_nonzero_exit_with_nothing_on_stdout_is_a_failure() {
    let dir = TempDir::new().unwrap();
    let err = runner().run(&sh("echo 'not logged in' >&2; exit 3", dir.path()))
                      .expect_err("a command that only failed");

    match err {
        ProbeError::Command { output, code, .. } => {
            assert_eq!(code, 3);
            assert!(output.contains("not logged in"),
                    "stderr is the message when stdout is empty");
        }
        other => panic!("got {other:?}"),
    }
}

/// `gemini mcp list` writes its whole listing to stderr and leaves stdout
/// empty. The shape was captured through `2>&1`, so nothing noticed until the
/// probe ran against the real binary and came back unreadable.
#[test]
fn a_listing_written_to_stderr_is_still_read() {
    let dir = TempDir::new().unwrap();
    let out = runner().run(&sh("echo 'github: https://x - Connected' >&2", dir.path()))
                      .expect("a successful command");

    assert!(out.contains("github"),
            "the listing on stderr was discarded");
}

/// A CLI that lists on stdout and warns on stderr must be read from stdout,
/// not from both blended together - a warning line in the middle of a listing
/// is not a server.
#[test]
fn stdout_wins_when_both_streams_have_something() {
    let dir = TempDir::new().unwrap();
    let out = runner()
        .run(&sh("echo 'warning: untrusted folder' >&2; echo 'github: https://x - Connected'",
                 dir.path()))
        .expect("a successful command");

    assert!(out.contains("github"));
    assert!(!out.contains("untrusted"),
            "stderr leaked into a successful listing");
}

#[test]
fn a_label_names_the_whole_command() {
    let dir = TempDir::new().unwrap();
    let command = ProbeCommand::new("claude",
                                    vec!["mcp".to_owned(), "list".to_owned()],
                                    dir.path());

    assert_eq!(command.label(), "claude mcp list");
    assert_eq!(ProbeCommand::new("claude", vec![], dir.path()).label(),
               "claude");
}

/// An agent CLI that health-checks as it lists can exit and leave a server it
/// started holding stdout open. The probe must end at the timeout, not when
/// that server does.
#[test]
fn a_server_holding_the_output_does_not_outlast_the_timeout() {
    let dir = TempDir::new().unwrap();
    let started = Instant::now();

    let err = runner_with(Duration::from_millis(200)).run(&sh("sleep 30 & echo listed",
                                                              dir.path()))
                                                     .unwrap_err();

    assert!(started.elapsed() < Duration::from_secs(5),
            "waited {:?} on a pipe the server held",
            started.elapsed());
    assert!(matches!(err, ProbeError::TimedOut { .. }), "got {err:?}");
}

/// The decision behind the stderr test, without a subprocess: a successful
/// run that wrote nothing to stdout is read from stderr (`gemini mcp list`).
#[test]
fn a_successful_run_with_an_empty_stdout_is_read_from_stderr() {
    let listing = "github: https://x - Connected\n".to_owned();

    assert_eq!(outcome("gemini mcp list",
                       true,
                       Some(0),
                       "  \n".to_owned(),
                       listing.clone()).expect("a successful run"),
               listing);
}

#[test]
fn a_successful_run_is_read_from_stdout_when_it_has_anything() {
    let out = outcome("claude mcp list",
                      true,
                      Some(0),
                      "github: https://x - Connected".to_owned(),
                      "warning: untrusted folder".to_owned()).expect("a successful run");

    assert!(out.contains("github") && !out.contains("untrusted"),
            "{out}");
}

#[test]
fn a_failed_run_that_still_listed_on_stdout_is_the_listing() {
    let out = outcome("claude mcp list",
                      false,
                      Some(1),
                      "github: https://x - connected".to_owned(),
                      "1 server failed".to_owned()).expect("output survives a non-zero exit");

    assert!(out.contains("github"));
}

#[test]
fn a_failed_run_with_an_empty_stdout_reports_stderr_and_its_code() {
    let err = outcome("claude mcp list",
                      false,
                      Some(3),
                      String::new(),
                      "not logged in\n".to_owned()).expect_err("a run that only failed");

    match err {
        ProbeError::Command { program,
                              output,
                              code, } => {
            assert_eq!(program, "claude mcp list");
            assert_eq!(output, "not logged in");
            assert_eq!(code, 3);
        }
        other => panic!("got {other:?}"),
    }
}

/// A child killed by a signal has no exit code; the error still has one.
#[test]
fn a_failed_run_without_an_exit_code_reports_minus_one() {
    let err = outcome("x", false, None, String::new(), String::new()).unwrap_err();

    assert!(matches!(err, ProbeError::Command { code: -1, .. }),
            "got {err:?}");
}

//! The programs a repository can configure for git to run, and which of them
//! run when Knot calls git.
//!
//! Knot reads a repository's status and diffs on its own, whenever a window
//! is open, so those calls run with no filesystem monitor, no hooks, no
//! external diff and no textconv. A commit is the user's own action, and the
//! repository's commit hooks run for it as they would in a terminal.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use knot_git::{CallOrigin, Repository, Runner};

fn init_repo(dir: &Path) {
    let run = |args: &[&str]| {
        Runner::new(dir).run(args).unwrap();
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
}

/// An executable script at `path` that creates `marker` when it runs.
fn marking_script(path: &Path, marker: &Path) {
    fs::write(path,
              format!("#!/bin/sh\ntouch '{}'\nexit 0\n", marker.display())).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A repository with one committed file and an uncommitted change to it,
/// plus a directory outside it for scripts and markers.
fn repo_with_a_change() -> (tempfile::TempDir, tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    fs::write(dir.path().join("f.txt"), "one\n").unwrap();
    let run = |args: &[&str]| Runner::new(dir.path()).run(args).unwrap();
    run(&["add", "f.txt"]);
    run(&["commit", "-qm", "root"]);
    fs::write(dir.path().join("f.txt"), "two\n").unwrap();
    let repo = Repository::open(dir.path());
    (dir, tools, repo)
}

fn marker(tools: &tempfile::TempDir, name: &str) -> PathBuf {
    tools.path().join(name)
}

#[test]
fn a_configured_filesystem_monitor_does_not_run_on_status() {
    let (dir, tools, repo) = repo_with_a_change();
    let ran = marker(&tools, "fsmonitor-ran");
    let script = tools.path().join("fsmonitor.sh");
    marking_script(&script, &ran);
    Runner::new(dir.path()).run(&["config", "core.fsmonitor", script.to_str().unwrap()])
                           .unwrap();

    let status = repo.status().unwrap();

    assert!(!status.is_clean(), "the change is still seen");
    assert!(!ran.exists(), "the monitor did not run");
}

#[test]
fn a_configured_external_diff_and_textconv_do_not_run_on_a_diff() {
    let (dir, tools, repo) = repo_with_a_change();
    let external = marker(&tools, "external-ran");
    let textconv = marker(&tools, "textconv-ran");
    marking_script(&tools.path().join("external.sh"), &external);
    marking_script(&tools.path().join("textconv.sh"), &textconv);
    let run = |args: &[&str]| Runner::new(dir.path()).run(args).unwrap();
    run(&["config",
          "diff.external",
          tools.path().join("external.sh").to_str().unwrap()]);
    run(&["config",
          "diff.text.textconv",
          tools.path().join("textconv.sh").to_str().unwrap()]);
    fs::write(dir.path().join(".gitattributes"), "*.txt diff=text\n").unwrap();

    let diff = repo.file_diff("f.txt", None, false).unwrap();
    let stats = repo.diff_stats().unwrap();

    assert!(diff.is_some(), "git's own diff is read");
    assert!(stats.files_changed > 0);
    assert!(!external.exists(), "the external diff did not run");
    assert!(!textconv.exists(), "the textconv filter did not run");
}

#[test]
fn staging_runs_no_hook() {
    let (dir, tools, repo) = repo_with_a_change();
    let ran = marker(&tools, "hook-ran");
    marking_script(&dir.path().join(".git/hooks/post-index-change"), &ran);

    repo.stage(&["f.txt"]).unwrap();

    assert!(!ran.exists(), "the index hook did not run");
}

/// The user's own commit runs their commit hooks, as it would in a terminal.
#[test]
fn a_commit_runs_the_repositorys_commit_hook() {
    let (dir, tools, repo) = repo_with_a_change();
    let ran = marker(&tools, "pre-commit-ran");
    marking_script(&dir.path().join(".git/hooks/pre-commit"), &ran);
    repo.stage(&["f.txt"]).unwrap();

    repo.commit("change f").unwrap();

    assert!(ran.exists(), "the pre-commit hook ran");
}

#[test]
fn a_path_that_starts_with_a_dash_is_staged_as_a_path() {
    let (dir, _tools, repo) = repo_with_a_change();
    fs::write(dir.path().join("-dash.txt"), "x\n").unwrap();

    repo.stage(&["-dash.txt"]).unwrap();

    let status = repo.status().unwrap();
    assert!(status.staged()
                  .any(|entry| entry.path == Path::new("-dash.txt")),
            "the file is staged");
}

/// A worktree add that ran `post-checkout` or not, for `origin`.
fn post_checkout_ran_for(origin: CallOrigin) -> bool {
    let (dir, tools, repo) = repo_with_a_change();
    let ran = marker(&tools, "post-checkout-ran");
    marking_script(&dir.path().join(".git/hooks/post-checkout"), &ran);

    repo.create_worktree("feature", &tools.path().join("worktree"), origin)
        .unwrap();

    ran.exists()
}

/// A worktree the user starts runs the repository's checkout hook (an LFS
/// `post-checkout`, say), as it would in a terminal.
#[test]
fn a_worktree_the_user_adds_runs_the_checkout_hook() {
    assert!(post_checkout_ran_for(CallOrigin::User));
}

/// A worktree an agent asks for through a tool runs no hook: the same
/// arguments, a different origin.
#[test]
fn a_worktree_added_for_an_agent_runs_no_hook() {
    assert!(!post_checkout_ran_for(CallOrigin::Automated));
}

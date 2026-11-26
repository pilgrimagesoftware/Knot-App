//! Repository trust: git does not run in a repository whose configuration
//! names programs until the policy trusts it with exactly those programs.
//!
//! The policy is process-wide and set once, so every test here shares one,
//! keyed by repository: each test works in its own temporary repository.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use knot_git::trust::repository_programs;
use knot_git::{GitError, Repository, RepositoryPrograms, Runner, TrustPolicy, configure_trust};

static TRUSTED: Mutex<Vec<RepositoryPrograms>> = Mutex::new(Vec::new());
static REFUSED: Mutex<Vec<RepositoryPrograms>> = Mutex::new(Vec::new());

struct Recorded;

impl TrustPolicy for Recorded {
    fn is_trusted(&self, programs: &RepositoryPrograms) -> bool {
        TRUSTED.lock().unwrap().contains(programs)
    }

    fn refused(&self, programs: &RepositoryPrograms) {
        REFUSED.lock().unwrap().push(programs.clone());
    }
}

fn install_policy() {
    // Every test calls this; only the first one sets it.
    configure_trust(Box::new(Recorded));
}

fn refusals_for(repository: &Path) -> usize {
    let repository = fs::canonicalize(repository).unwrap();
    REFUSED.lock()
           .unwrap()
           .iter()
           .filter(|programs| programs.repository == repository)
           .count()
}

fn trust(folder: &Path) {
    let programs = repository_programs(folder).unwrap().expect("a repository");
    TRUSTED.lock().unwrap().push(programs);
}

/// A repository with `f.txt` committed and changed since, whose
/// `.gitattributes` routes it through filter `x` - not yet configured. The
/// second folder is outside it, for markers.
fn repository() -> (tempfile::TempDir, tempfile::TempDir) {
    install_policy();
    let dir = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        Runner::new(dir.path()).run(args).unwrap();
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
    fs::write(dir.path().join(".gitattributes"), "f.txt filter=x\n").unwrap();
    fs::write(dir.path().join("f.txt"), "one\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-qm", "root"]);
    fs::write(dir.path().join("f.txt"), "two\n").unwrap();
    (dir, tools)
}

/// Configures filter `x`'s `clean` to create `marker` and pass the content
/// through - set by the test, through git, while nothing is configured yet.
fn configure_clean_filter(dir: &Path, marker: &Path) {
    Runner::new(dir).run(&["config",
                           "filter.x.clean",
                           &format!("touch '{}'; cat", marker.display())])
                    .unwrap();
}

fn marker(tools: &tempfile::TempDir) -> PathBuf {
    tools.path().join("clean-ran")
}

#[test]
fn an_untrusted_clean_filter_does_not_run_and_the_refusal_is_reported() {
    let (dir, tools) = repository();
    let ran = marker(&tools);
    configure_clean_filter(dir.path(), &ran);

    let error = Repository::open(dir.path()).stage(&["f.txt"]).unwrap_err();

    assert!(matches!(error, GitError::Untrusted { .. }), "{error}");
    assert!(!ran.exists(), "the clean filter ran");
    assert!(refusals_for(dir.path()) > 0);
}

#[test]
fn a_trusted_clean_filter_runs() {
    let (dir, tools) = repository();
    let ran = marker(&tools);
    configure_clean_filter(dir.path(), &ran);

    trust(dir.path());
    Repository::open(dir.path()).stage(&["f.txt"]).unwrap();

    assert!(ran.exists(), "the clean filter ran once trusted");
}

/// Trust covers the programs it was given: one added afterwards - here
/// straight into `.git/config`, as a pulled include or another tool might -
/// is asked about again.
#[test]
fn a_program_added_after_trust_is_refused_until_trusted_again() {
    let (dir, tools) = repository();
    let ran = marker(&tools);
    configure_clean_filter(dir.path(), &ran);
    trust(dir.path());
    let config = dir.path().join(".git/config");
    let mut text = fs::read_to_string(&config).unwrap();
    text.push_str("[merge \"x\"]\n\tdriver = cat %A\n");
    fs::write(&config, text).unwrap();

    let error = Repository::open(dir.path()).stage(&["f.txt"]).unwrap_err();

    assert!(matches!(error, GitError::Untrusted { .. }), "{error}");
    assert!(!ran.exists(), "the clean filter ran");
}

/// A hook defined in configuration runs even with `core.hooksPath` set to
/// `/dev/null`, so it counts as a program too.
#[test]
fn a_hook_defined_in_configuration_is_refused() {
    let (dir, tools) = repository();
    let ran = tools.path().join("hook-ran");
    let run = |args: &[&str]| {
        Runner::new(dir.path()).run(args).unwrap();
    };
    run(&["config", "hook.mark.event", "post-index-change"]);
    run(&["config",
          "hook.mark.command",
          &format!("touch '{}'", ran.display())]);

    let error = Repository::open(dir.path()).status().unwrap_err();

    assert!(matches!(error, GitError::Untrusted { .. }), "{error}");
    assert!(!ran.exists(), "the configured hook ran");
}

/// Included files are the repository's configuration too.
#[test]
fn a_filter_in_an_included_file_is_refused() {
    let (dir, tools) = repository();
    let ran = marker(&tools);
    fs::write(dir.path().join("extra.cfg"),
              format!("[filter \"x\"]\n\tclean = touch '{}'; cat\n", ran.display())).unwrap();
    Runner::new(dir.path()).run(&["config", "include.path", "../extra.cfg"])
                           .unwrap();

    let error = Repository::open(dir.path()).stage(&["f.txt"]).unwrap_err();

    assert!(matches!(error, GitError::Untrusted { .. }), "{error}");
    assert!(!ran.exists(), "the included clean filter ran");
}

#[test]
fn a_repository_that_names_no_programs_is_not_checked_by_the_policy() {
    let (dir, _tools) = repository();
    // Filter `x` is named in `.gitattributes` but configured nowhere.
    let programs = repository_programs(dir.path()).unwrap()
                                                  .expect("a repository");

    Repository::open(dir.path()).stage(&["f.txt"]).unwrap();

    assert!(programs.settings.is_empty(), "{:?}", programs.settings);
    assert_eq!(refusals_for(dir.path()), 0);
}

#[test]
fn a_folder_outside_any_repository_has_no_repository_programs() {
    install_policy();
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(repository_programs(dir.path()).unwrap(), None);
}

/// A submodule's configuration lives in its own git folder, outside the
/// repository's `local` scope, yet the repository's `status` runs git in
/// the submodule: its clean filter is the repository's to answer for.
#[test]
fn a_submodules_clean_filter_does_not_run_until_trusted() {
    install_policy();
    let dir = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let ran = marker(&tools);
    let git = |folder: &Path, args: &[&str]| {
        Runner::new(folder).run(args).unwrap();
    };
    let init = |folder: &Path| {
        fs::create_dir_all(folder).unwrap();
        git(folder, &["init", "-q", "-b", "main"]);
        git(folder, &["config", "user.email", "test@example.com"]);
        git(folder, &["config", "user.name", "Test"]);
        git(folder, &["config", "commit.gpgsign", "false"]);
    };
    let (library, app) = (dir.path().join("library"), dir.path().join("app"));
    init(&library);
    fs::write(library.join(".gitattributes"), "f.txt filter=x\n").unwrap();
    fs::write(library.join("f.txt"), "one\n").unwrap();
    git(&library, &["add", "."]);
    git(&library, &["commit", "-qm", "root"]);
    init(&app);
    fs::write(app.join("main.txt"), "main\n").unwrap();
    git(&app, &["add", "."]);
    git(&app, &["commit", "-qm", "root"]);
    git(&app,
        &["-c",
          "protocol.file.allow=always",
          "submodule",
          "add",
          "-q",
          library.to_str().unwrap(),
          "vendor/library"]);
    git(&app, &["commit", "-qm", "add submodule"]);
    let checkout = app.join("vendor/library");
    // Same size, new content: git has to read the file to compare it.
    fs::write(checkout.join("f.txt"), "two\n").unwrap();
    configure_clean_filter(&checkout, &ran);

    let error = Repository::open(&app).status().unwrap_err();

    assert!(matches!(error, GitError::Untrusted { .. }), "{error}");
    assert!(!ran.exists(), "the submodule's clean filter ran");
    let programs = repository_programs(&app).unwrap().expect("a repository");
    assert_eq!(programs.settings[0].submodule.as_deref(),
               Some("vendor/library"));

    trust(&app);
    Repository::open(&app).status().unwrap();

    assert!(ran.exists(), "once trusted, the submodule's filter runs");
}

/// A hooks folder (husky, say) is turned off on every call Knot makes on
/// its own, so it alone asks nothing - and its hooks still do not run.
#[test]
fn a_repository_whose_only_program_is_a_hooks_folder_is_not_asked_about() {
    let (dir, tools) = repository();
    let ran = tools.path().join("husky-ran");
    let hooks = dir.path().join(".husky");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("post-index-change");
    fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", ran.display())).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    Runner::new(dir.path()).run(&["config", "core.hooksPath", ".husky"])
                           .unwrap();

    Repository::open(dir.path()).stage(&["f.txt"]).unwrap();

    assert_eq!(refusals_for(dir.path()), 0);
    assert!(!ran.exists(),
            "the hooks folder is still off for Knot's calls");
}

/// A textconv is turned off on the diffs Knot reads, so it alone asks
/// nothing - and the diff is read without running it.
#[test]
fn a_repository_whose_only_program_is_a_textconv_is_not_asked_about() {
    let (dir, tools) = repository();
    let ran = tools.path().join("textconv-ran");
    fs::write(dir.path().join(".gitattributes"), "f.txt diff=x\n").unwrap();
    Runner::new(dir.path()).run(&["config",
                                  "diff.x.textconv",
                                  &format!("sh -c 'touch {}; cat \"$1\"' -", ran.display())])
                           .unwrap();

    let diff = Repository::open(dir.path()).file_diff("f.txt", None, false);

    assert!(diff.is_ok(), "{diff:?}");
    assert_eq!(refusals_for(dir.path()), 0);
    assert!(!ran.exists(), "the textconv ran");
}

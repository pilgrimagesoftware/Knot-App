//! `Repository::list_files` reaches into submodules (#312): what the @-file
//! picker offers is every file in the working tree, not only the
//! superproject's.

use std::path::Path;

use knot_git::{Repository, Runner};

fn git(dir: &Path, args: &[&str]) {
    Runner::new(dir).run(args).unwrap();
}

/// A repository at `dir` with one committed file, `name`.
fn repo(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join(name), "content\n").unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "init"]);
}

/// Adds the repository at `source` to `superproject` as a submodule at
/// `path`, and commits it. Local paths need the file transport allowed.
fn add_submodule(superproject: &Path, source: &Path, path: &str) {
    git(superproject,
        &["-c",
          "protocol.file.allow=always",
          "submodule",
          "add",
          "-q",
          source.to_str().unwrap(),
          path]);
    git(superproject, &["commit", "-qm", "add submodule"]);
}

fn sorted(mut files: Vec<String>) -> Vec<String> {
    files.sort();
    files
}

#[test]
fn a_submodules_files_are_listed_under_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library");
    let app = dir.path().join("app");
    repo(&library, "lib.rs");
    repo(&app, "main.rs");
    add_submodule(&app, &library, "vendor/library");

    let files = sorted(Repository::open(&app).list_files().unwrap());

    assert_eq!(files,
               [".gitmodules", "main.rs", "vendor/library/lib.rs"],
               "the submodule's file, and not its directory as an entry");
}

/// Untracked files inside a submodule are offered as they are in the
/// superproject - git's own `--recurse-submodules` cannot list them - and
/// ignored ones are not.
#[test]
fn a_submodules_untracked_files_are_listed_and_its_ignored_ones_are_not() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library");
    let app = dir.path().join("app");
    repo(&library, "lib.rs");
    repo(&app, "main.rs");
    add_submodule(&app, &library, "library");
    let checkout = app.join("library");
    std::fs::write(checkout.join(".gitignore"), "build.log\n").unwrap();
    std::fs::write(checkout.join("notes.md"), "draft\n").unwrap();
    std::fs::write(checkout.join("build.log"), "noise\n").unwrap();

    let files = Repository::open(&app).list_files().unwrap();

    assert!(files.contains(&"library/notes.md".to_owned()), "{files:?}");
    assert!(!files.contains(&"library/build.log".to_owned()),
            "{files:?}");
}

#[test]
fn a_nested_submodules_files_are_listed_under_both_paths() {
    let dir = tempfile::tempdir().unwrap();
    let inner = dir.path().join("inner");
    let middle = dir.path().join("middle");
    let app = dir.path().join("app");
    repo(&inner, "deep.rs");
    repo(&middle, "mid.rs");
    add_submodule(&middle, &inner, "inner");
    repo(&app, "main.rs");
    // A clone of `middle` carrying its own submodule checked out.
    git(&app,
        &["-c",
          "protocol.file.allow=always",
          "submodule",
          "add",
          "-q",
          middle.to_str().unwrap(),
          "middle"]);
    git(&app.join("middle"),
        &["-c",
          "protocol.file.allow=always",
          "submodule",
          "update",
          "-q",
          "--init"]);
    git(&app, &["commit", "-qm", "add middle"]);

    let files = Repository::open(&app).list_files().unwrap();

    assert!(files.contains(&"middle/mid.rs".to_owned()), "{files:?}");
    assert!(files.contains(&"middle/inner/deep.rs".to_owned()),
            "{files:?}");
}

/// A submodule that has not been checked out is an empty directory: it has
/// nothing to offer, is not an error, and is not recursed into.
#[test]
fn a_submodule_that_is_not_checked_out_lists_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("library");
    let app = dir.path().join("app");
    let clone = dir.path().join("clone");
    repo(&library, "lib.rs");
    repo(&app, "main.rs");
    add_submodule(&app, &library, "library");
    // Cloned without `--recurse-submodules`, so `library/` stays empty.
    git(dir.path(),
        &["clone",
          "-q",
          app.to_str().unwrap(),
          clone.to_str().unwrap()]);

    let started = std::time::Instant::now();
    let files = sorted(Repository::open(&clone).list_files().unwrap());

    assert_eq!(files, [".gitmodules", "main.rs"]);
    // Inside the empty directory git falls back to the superproject, which
    // reports the same gitlink again as `./`. Recursing into it listed the
    // same directory under an ever longer path until git gave up, some
    // thirty seconds later, with the right answer - so the time is what
    // shows it. Normally a fraction of a second.
    assert!(started.elapsed() < std::time::Duration::from_secs(10),
            "listing recursed into a submodule that is not checked out");
}

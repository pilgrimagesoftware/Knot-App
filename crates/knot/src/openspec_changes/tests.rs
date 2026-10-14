//! Unit tests for [`super`]: one test per scenario under "Un-archived
//! changes are found in the workspace's folders".

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{ChangeEntry, merge, scan};

/// `<worktree>/openspec/changes/<name>[/proposal.md]`.
fn write_change(worktree: &Path, name: &str, proposal: Option<&str>) {
    let dir = worktree.join("openspec").join("changes").join(name);
    fs::create_dir_all(&dir).expect("create change dir");
    if let Some(proposal) = proposal {
        fs::write(dir.join("proposal.md"), proposal).expect("write proposal.md");
    }
}

#[test]
fn lists_a_change() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);

    let changes = scan(worktree.path());

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].name, "add-login");
    assert_eq!(changes[0].dir,
               worktree.path()
                       .join("openspec")
                       .join("changes")
                       .join("add-login"));
}

#[test]
fn excludes_the_archive_directory() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);
    write_change(worktree.path(), "archive", None);
    write_change(&worktree.path()
                          .join("openspec")
                          .join("changes")
                          .join("archive"),
                 "2026-09-01-old-work",
                 None);

    let changes = scan(worktree.path());

    assert_eq!(changes.iter()
                      .map(|change| change.name.as_str())
                      .collect::<Vec<_>>(),
               vec!["add-login"]);
}

#[test]
fn excludes_dot_directories() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);
    write_change(worktree.path(), ".DS_Store_dir", None);

    let changes = scan(worktree.path());

    assert_eq!(changes.iter()
                      .map(|change| change.name.as_str())
                      .collect::<Vec<_>>(),
               vec!["add-login"]);
}

#[test]
fn ignores_plain_files() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);
    fs::write(worktree.path()
                      .join("openspec")
                      .join("changes")
                      .join("README.md"),
              "notes").expect("write file");

    let changes = scan(worktree.path());

    assert_eq!(changes.iter()
                      .map(|change| change.name.as_str())
                      .collect::<Vec<_>>(),
               vec!["add-login"]);
}

#[test]
fn a_missing_changes_directory_yields_an_empty_list() {
    let worktree = TempDir::new().expect("tempdir");

    assert_eq!(scan(worktree.path()), Vec::new());
}

#[test]
fn why_is_the_first_non_empty_line_after_the_heading() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(),
                 "add-login",
                 Some("# Add Login\n\n## Why\n\nUsers cannot sign in with SSO.\n\nMore detail.\n\n## What Changes\n\nSomething else.\n"));

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why,
               Some("Users cannot sign in with SSO.".to_string()));
}

#[test]
fn why_is_none_with_no_proposal() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why, None);
}

#[test]
fn why_is_none_with_no_heading() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(),
                 "add-login",
                 Some("# Add Login\n\nNo Why section here.\n"));

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why, None);
}

#[test]
fn why_is_none_with_an_empty_heading() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(),
                 "add-login",
                 Some("# Add Login\n\n## Why\n\n## What Changes\n\nSomething else.\n"));

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why, None);
}

#[test]
fn a_change_in_two_worktrees_merges_to_one_entry_keeping_the_first() {
    let first = TempDir::new().expect("tempdir");
    let second = TempDir::new().expect("tempdir");
    write_change(first.path(), "add-login", None);
    write_change(second.path(), "add-login", None);

    let merged = merge([scan(first.path()), scan(second.path())]);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].dir,
               first.path()
                    .join("openspec")
                    .join("changes")
                    .join("add-login"));
}

#[test]
fn merge_keeps_changes_unique_to_either_worktree() {
    let first = TempDir::new().expect("tempdir");
    let second = TempDir::new().expect("tempdir");
    write_change(first.path(), "add-login", None);
    write_change(second.path(), "add-export", None);

    let merged = merge([scan(first.path()), scan(second.path())]);

    assert_eq!(merged.iter()
                     .map(|change: &ChangeEntry| change.name.as_str())
                     .collect::<Vec<_>>(),
               vec!["add-export", "add-login"]);
}

/// The `(subfolder, name)` of each change `scan` finds, in its order.
fn found(worktree: &Path) -> Vec<(Option<String>, String)> {
    scan(worktree).into_iter()
                  .map(|change| {
                      (change.subfolder
                             .map(|subfolder| subfolder.display().to_string()),
                       change.name)
                  })
                  .collect()
}

#[test]
fn finds_a_change_in_a_subfolder() {
    let worktree = TempDir::new().expect("tempdir");
    let api = worktree.path().join("services").join("api");
    write_change(&api, "add-login", None);

    let changes = scan(worktree.path());

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].name, "add-login");
    assert_eq!(changes[0].subfolder,
               Some(Path::new("services").join("api")));
    assert_eq!(changes[0].dir,
               api.join("openspec").join("changes").join("add-login"));
}

#[test]
fn lists_the_root_first_then_each_subfolder() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "zz-root", None);
    write_change(&worktree.path().join("web"), "add-login", None);
    write_change(&worktree.path().join("api"), "add-login", None);

    assert_eq!(found(worktree.path()),
               vec![(None, "zz-root".to_string()),
                    (Some("api".to_string()), "add-login".to_string()),
                    (Some("web".to_string()), "add-login".to_string()),]);
}

#[test]
fn a_subfolder_archive_is_excluded() {
    let worktree = TempDir::new().expect("tempdir");
    let api = worktree.path().join("api");
    write_change(&api, "add-login", None);
    write_change(&api.join("openspec").join("changes").join("archive"),
                 "2026-09-01-old-work",
                 None);

    assert_eq!(found(worktree.path()),
               vec![(Some("api".to_string()), "add-login".to_string())]);
}

#[test]
fn skips_dependency_hidden_and_nested_repository_folders() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(&worktree.path().join("node_modules").join("some-pkg"),
                 "vendored",
                 None);
    write_change(&worktree.path().join(".cache"), "hidden", None);
    let nested = worktree.path().join("nested-repo");
    write_change(&nested, "its-own", None);
    fs::create_dir_all(nested.join(".git")).expect("create nested .git");

    assert_eq!(found(worktree.path()), Vec::new());
}

#[test]
fn stops_at_the_maximum_depth() {
    let worktree = TempDir::new().expect("tempdir");
    let mut deep = worktree.path().to_path_buf();
    for level in 0..crate::consts::OPENSPEC_SCAN_MAX_DEPTH {
        deep = deep.join(format!("level{level}"));
    }
    write_change(&deep, "deepest-found", None);
    write_change(&deep.join("too-deep"), "not-found", None);

    assert_eq!(found(worktree.path()).into_iter()
                                     .map(|(_, name)| name)
                                     .collect::<Vec<_>>(),
               vec!["deepest-found"]);
}

#[test]
fn merge_keeps_one_name_in_two_subfolders_as_two_changes() {
    let first = TempDir::new().expect("tempdir");
    let second = TempDir::new().expect("tempdir");
    write_change(&first.path().join("api"), "add-login", None);
    write_change(&second.path().join("api"), "add-login", None);
    write_change(&second.path().join("web"), "add-login", None);

    let merged = merge([scan(first.path()), scan(second.path())]);

    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].dir,
               first.path()
                    .join("api")
                    .join("openspec")
                    .join("changes")
                    .join("add-login"));
    assert_eq!(merged[1].subfolder, Some(Path::new("web").to_path_buf()));
}

#[test]
fn prompt_ref_is_the_name_at_the_root_and_the_path_in_a_subfolder() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-export", None);
    write_change(&worktree.path().join("api"), "add-login", None);

    let changes = scan(worktree.path());

    assert_eq!(changes[0].prompt_ref(), "add-export");
    assert_eq!(changes[1].prompt_ref(),
               Path::new("api").join("openspec")
                               .join("changes")
                               .join("add-login")
                               .display()
                               .to_string());
}

//! Finds the folders of a working tree that hold an `openspec/changes/`
//! directory: the root, and any subfolder up to
//! [`OPENSPEC_SCAN_MAX_DEPTH`] below it (#203), so a monorepo's
//! `services/api/openspec/changes/` is found as well as the root's.
//!
//! What the walk does not enter: folders starting with `.` (`.git` among
//! them), [`OPENSPEC_SCAN_SKIPPED_DIRS`], an `openspec` folder (its changes
//! are read by the caller, its insides are not another project), symlinks,
//! and a folder holding its own `.git` - a nested repository or submodule,
//! which Knot scans as a repository of its own when an agent works in it.

use std::fs;
use std::path::{Path, PathBuf};

use crate::consts::{OPENSPEC_SCAN_MAX_DEPTH, OPENSPEC_SCAN_SKIPPED_DIRS};

/// Each folder under `worktree` holding `openspec/changes/`, relative to
/// `worktree` - the empty path for the root itself. Unordered; the caller
/// sorts what it scans.
pub(super) fn openspec_roots(worktree: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut pending = vec![(PathBuf::new(), 0_usize)];
    while let Some((relative, depth)) = pending.pop() {
        let dir = worktree.join(&relative);
        if dir.join("openspec").join("changes").is_dir() {
            roots.push(relative.clone());
        }
        if depth == OPENSPEC_SCAN_MAX_DEPTH {
            continue;
        }
        pending.extend(walkable_children(&dir).map(|name| (relative.join(name), depth + 1)));
    }
    roots
}

/// The names of `dir`'s subfolders the walk enters, per the module doc.
fn walkable_children(dir: &Path) -> impl Iterator<Item = String> {
    fs::read_dir(dir).into_iter()
                     .flatten()
                     .filter_map(Result::ok)
                     // `file_type` does not follow symlinks, so a link cannot
                     // loop the walk or lead it out of the working tree.
                     .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                     .filter(|entry| !entry.path().join(".git").exists())
                     .filter_map(|entry| entry.file_name().into_string().ok())
                     .filter(|name| {
                         !name.starts_with('.')
                         && name != "openspec"
                         && !OPENSPEC_SCAN_SKIPPED_DIRS.contains(&name.as_str())
                     })
}

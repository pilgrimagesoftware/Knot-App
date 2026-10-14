//! Scans a working tree's `openspec/changes/` directories - the root's and
//! any subfolder's (#203) - for un-archived OpenSpec changes, and merges
//! results from a repository's several worktrees into one list.
//!
//! Contract: "Un-archived changes are found in the workspace's folders" in
//! the `workspace-openspec-changes` capability spec under `openspec/specs/`.
//!
//! Pure file-system reads, no git: `roots` finds the folders holding an
//! `openspec/changes/`, [`scan`] reads each with [`std::fs::read_dir`], and
//! [`merge`] is a flat dedupe by subfolder and name. The window
//! wires both into the OpenSpec tab's cache and `spawn_blocking`.
//!
//! `filter` holds the search and repository filtering the tab applies to the
//! rows this module scans.

pub(crate) mod filter;
mod roots;

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use roots::openspec_roots;

/// One un-archived OpenSpec change found in a working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangeEntry {
    pub(crate) name:      String,
    pub(crate) dir:       PathBuf,
    /// The folder holding the `openspec/` directory the change is in,
    /// relative to the working tree: `None` for the root's own, e.g.
    /// `Some("services/api")` for a monorepo package's.
    pub(crate) subfolder: Option<PathBuf>,
    /// The first non-empty line after a `## Why` heading in the change's
    /// `proposal.md`, trimmed. `None` when there is no `proposal.md`, no
    /// such heading, or nothing before the next `##` heading.
    pub(crate) why:       Option<String>,
}

impl ChangeEntry {
    /// How a prompt to an agent names the change: its name for the root's,
    /// and its path from the working tree for a subfolder's, since the
    /// name alone does not say which `openspec/` to look in.
    pub(crate) fn prompt_ref(&self) -> String {
        match &self.subfolder {
            None => self.name.clone(),
            Some(subfolder) => subfolder.join("openspec")
                                        .join("changes")
                                        .join(&self.name)
                                        .display()
                                        .to_string(),
        }
    }

    /// The tab's order: the root's changes first, then each subfolder's,
    /// by name within each.
    fn order(&self, other: &Self) -> Ordering {
        self.subfolder
            .cmp(&other.subfolder)
            .then_with(|| self.name.cmp(&other.name))
    }

    fn same_change(&self, other: &Self) -> bool {
        self.subfolder == other.subfolder && self.name == other.name
    }
}

/// The un-archived changes directly under each `openspec/changes/` in
/// `worktree` - the root's and those `roots` finds in its subfolders -
/// sorted root first, then by subfolder, then by name.
///
/// A directory entry named `archive` or starting with `.` is skipped, and so
/// is anything that is not a directory. A working tree with no
/// `openspec/changes/` anywhere is not an error: it yields an empty list,
/// the same as one whose are all empty.
pub(crate) fn scan(worktree: &Path) -> Vec<ChangeEntry> {
    let mut changes: Vec<ChangeEntry> =
        openspec_roots(worktree).into_iter()
                                .flat_map(|relative| scan_changes_dir(worktree, relative))
                                .collect();
    changes.sort_by(ChangeEntry::order);
    changes
}

/// The changes in `<worktree>/<relative>/openspec/changes/`.
fn scan_changes_dir(worktree: &Path, relative: PathBuf) -> Vec<ChangeEntry> {
    let changes_dir = worktree.join(&relative).join("openspec").join("changes");
    let Ok(entries) = fs::read_dir(&changes_dir)
    else {
        return Vec::new();
    };
    let subfolder = (!relative.as_os_str().is_empty()).then_some(relative);

    entries.filter_map(Result::ok)
           .filter(|entry| entry.path().is_dir())
           .filter_map(|entry| {
               let name = entry.file_name().into_string().ok()?;
               if name == "archive" || name.starts_with('.') {
                   return None;
               }
               let dir = entry.path();
               let why = why_line(&dir);
               Some(ChangeEntry { name,
                                  dir,
                                  subfolder: subfolder.clone(),
                                  why })
           })
           .collect()
}

/// The `## Why` line a change's `proposal.md` summarises itself with, per
/// [`ChangeEntry::why`].
fn why_line(dir: &Path) -> Option<String> {
    let proposal = fs::read_to_string(dir.join("proposal.md")).ok()?;
    let mut lines = proposal.lines();
    lines.find(|line| line.trim() == "## Why")?;
    lines.map(str::trim)
         .take_while(|line| !line.starts_with("##"))
         .find(|line| !line.is_empty())
         .map(str::to_string)
}

/// Merges one repository's [`scan`] results from several worktrees into one
/// list, in [`scan`]'s order, keeping the first worktree's entry for a
/// change - the same name in the same subfolder - found in more than one,
/// so Reveal in Finder targets the worktree it was first seen in. The same
/// name in two subfolders is two changes.
pub(crate) fn merge(per_worktree: impl IntoIterator<Item = Vec<ChangeEntry>>) -> Vec<ChangeEntry> {
    let mut merged: Vec<ChangeEntry> = Vec::new();
    for changes in per_worktree {
        for change in changes {
            if !merged.iter().any(|existing| existing.same_change(&change)) {
                merged.push(change);
            }
        }
    }
    merged.sort_by(ChangeEntry::order);
    merged
}

//! Reading which of a folder's repository configuration names programs.
//!
//! Two git calls, neither of which runs a configured program: `rev-parse`
//! for where the repository keeps its configuration, then `config --list`
//! with `--show-scope` so only the repository's own entries - `local`, and
//! `worktree` for a linked worktree's `config.worktree` - are kept. The
//! user's global and system configuration is theirs and is not checked.
//! Files a repository includes report the including file's scope, so
//! `--includes` brings their entries in as the repository's.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::keys::{ProgramSetting, names_a_program};
use crate::consts::{CONFIG_LIST_WITH_ORIGIN, REV_PARSE_GIT_DIRS, REV_PARSE_TOPLEVEL};
use crate::error::{GitError, Result};
use crate::runner::Runner;

/// A repository and the configuration entries in it that name programs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryPrograms {
    /// The repository, canonical: the main worktree's folder, so every
    /// linked worktree of one repository answers with the same path.
    pub repository: PathBuf,
    /// Sorted and without duplicates. Empty when nothing is named.
    pub settings:   Vec<ProgramSetting>,
}

/// A file's modification time and length, or `None` while it is missing.
type Stamp = Option<(SystemTime, u64)>;

/// One folder's answer, with the files it was read from, so it can be
/// reused until one of them changes.
#[derive(Debug, Clone)]
pub(super) struct Detection {
    pub(super) programs: RepositoryPrograms,
    watched:             Vec<(PathBuf, Stamp)>,
}

impl Detection {
    /// Whether every file this answer was read from is as it was then.
    pub(super) fn is_current(&self) -> bool {
        self.watched
            .iter()
            .all(|(path, stamp)| stamp_of(path) == *stamp)
    }
}

fn stamp_of(path: &Path) -> Stamp {
    let metadata = fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

/// The repository `folder` is in and what its configuration names, or
/// `None` when `folder` is not in a repository.
pub(super) fn detect(folder: &Path) -> Result<Option<Detection>> {
    let runner = Runner::new(folder).without_trust_check();
    let dirs = match runner.run(REV_PARSE_GIT_DIRS) {
        Ok(dirs) => dirs,
        // Outside a repository (or one git refuses to open): there is no
        // repository configuration, and git will fail any call there itself.
        Err(GitError::Command { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut lines = dirs.lines();
    let (Some(common_dir), Some(git_dir)) = (lines.next(), lines.next())
    else {
        return Err(GitError::Parse(format!("unexpected rev-parse output: {dirs:?}")));
    };
    let (common_dir, git_dir) = (PathBuf::from(common_dir), PathBuf::from(git_dir));
    // `--show-origin` names files relative to the worktree's top, where git
    // runs from; with no worktree, from where it was started.
    let base = runner.run(REV_PARSE_TOPLEVEL)
                     .map_or_else(|_| folder.to_path_buf(), PathBuf::from);

    let listing = runner.run_raw(CONFIG_LIST_WITH_ORIGIN)?;
    let mut watched = vec![common_dir.join("config"), git_dir.join("config.worktree")];
    let mut settings = Vec::new();
    for entry in parse_listing(&listing)? {
        if !matches!(entry.scope, "local" | "worktree") {
            continue;
        }
        let origin = entry.origin
                          .strip_prefix("file:")
                          .map(|file| base.join(file));
        if let Some(origin) = &origin {
            watched.push(origin.clone());
            if is_include(entry.key) {
                watched.push(include_target(origin, entry.value));
            }
        }
        if names_a_program(entry.key, entry.value) {
            settings.push(ProgramSetting { key:   entry.key.to_owned(),
                                           value: entry.value.to_owned(), });
        }
    }
    settings.sort();
    settings.dedup();
    watched.sort();
    watched.dedup();
    let watched = watched.into_iter()
                         .map(|path| {
                             let stamp = stamp_of(&path);
                             (path, stamp)
                         })
                         .collect();
    Ok(Some(Detection { programs:
                            RepositoryPrograms { repository: repository_folder(&common_dir),
                                                 settings },
                        watched }))
}

struct Entry<'a> {
    scope:  &'a str,
    origin: &'a str,
    key:    &'a str,
    value:  &'a str,
}

/// `config --list --show-scope --show-origin -z`: per entry, the scope, the
/// origin and `key\nvalue` (just `key` for a value-less boolean), each
/// ended by a NUL.
fn parse_listing(listing: &str) -> Result<Vec<Entry<'_>>> {
    let fields: Vec<&str> = listing.split('\0').collect();
    let Some((last, fields)) = fields.split_last()
    else {
        return Ok(Vec::new());
    };
    let (entries, rest) = fields.as_chunks::<3>();
    if !last.is_empty() || !rest.is_empty() {
        return Err(GitError::Parse("unexpected config listing".to_owned()));
    }
    Ok(entries.iter()
              .map(|[scope, origin, key_value]| {
                  let (key, value) = key_value.split_once('\n').unwrap_or((key_value, ""));
                  Entry { scope,
                          origin,
                          key,
                          value }
              })
              .collect())
}

fn is_include(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key == "include.path" || (key.starts_with("includeif.") && key.ends_with(".path"))
}

/// Where an include points: relative to the including file's folder, or
/// under the home folder for `~/`. Watched even while it does not exist, so
/// a file that appears later is noticed.
fn include_target(origin: &Path, value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/")
       && let Some(home) = std::env::home_dir()
    {
        return home.join(rest);
    }
    origin.parent().unwrap_or(origin).join(value)
}

/// The folder a repository is known by: the main worktree's, for a
/// repository whose git folder is its `.git`; the git folder itself
/// otherwise.
fn repository_folder(common_dir: &Path) -> PathBuf {
    let common_dir = fs::canonicalize(common_dir).unwrap_or_else(|_| common_dir.to_path_buf());
    match (common_dir.file_name(), common_dir.parent()) {
        (Some(name), Some(parent)) if name == ".git" => parent.to_path_buf(),
        _ => common_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listing_parses_into_scoped_entries() {
        let listing =
            "global\0file:/home/u/.gitconfig\0user.name\nU\0local\0file:.git/config\0core.bare\0";

        let entries = parse_listing(listing).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!((entries[0].scope, entries[0].key, entries[0].value),
                   ("global", "user.name", "U"));
        assert_eq!((entries[1].scope, entries[1].key, entries[1].value),
                   ("local", "core.bare", ""));
    }

    #[test]
    fn a_truncated_listing_is_an_error() {
        assert!(parse_listing("local\0file:.git/config\0").is_err());
    }

    #[test]
    fn includes_are_recognised_and_resolved_beside_their_file() {
        assert!(is_include("include.path"));
        assert!(is_include("includeIf.gitdir:~/src/.path"));
        assert!(!is_include("filter.x.clean"));
        assert_eq!(include_target(Path::new("/r/.git/config"), "../x.cfg"),
                   Path::new("/r/.git/../x.cfg"));
        assert_eq!(include_target(Path::new("/r/.git/config"), "/abs.cfg"),
                   Path::new("/abs.cfg"));
    }
}

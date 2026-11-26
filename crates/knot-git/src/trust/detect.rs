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
use crate::consts::{
    CONFIG_FILE_REGEXP_PREFIX, CONFIG_LIST_WITH_ORIGIN, GITLINK_MODE, GITMODULES_FILE,
    LS_FILES_STAGE, MAX_SUBMODULE_DEPTH, REV_PARSE_GIT_DIRS, REV_PARSE_TOPLEVEL,
    SUBMODULE_PATH_PATTERN, UNREAD_SUBMODULE_KEY, UNREAD_SUBMODULE_VALUE,
};
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

/// The repository `folder` is in and what its configuration names - its
/// checked-out submodules' configuration included - or `None` when
/// `folder` is not in a repository.
pub(super) fn detect(folder: &Path) -> Result<Option<Detection>> {
    let Some(Found { common_dir,
                     mut settings,
                     mut watched, }) = read_repository(folder, 0, MAX_SUBMODULE_DEPTH)?
    else {
        return Ok(None);
    };
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

/// One repository's reading, before it is sorted and stamped.
struct Found {
    common_dir: PathBuf,
    settings:   Vec<ProgramSetting>,
    watched:    Vec<PathBuf>,
}

/// Reads `folder`'s repository, then each checked-out submodule's, down to
/// `max_depth` levels ([`MAX_SUBMODULE_DEPTH`] outside tests). A submodule's
/// configuration lives in its own git folder (`.git/modules/<name>/config`),
/// outside the repository's `local` scope, yet `status` and `diff` run git in
/// it - so its programs are the repository's to answer for too.
fn read_repository(folder: &Path, depth: usize, max_depth: usize) -> Result<Option<Found>> {
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
            settings.push(ProgramSetting { submodule: None,
                                           key:       entry.key.to_owned(),
                                           value:     entry.value.to_owned(), });
        }
    }

    let gitmodules = base.join(GITMODULES_FILE);
    watched.push(gitmodules.clone());
    // `status` enters a populated gitlink whether `.gitmodules` names it or
    // not, so the index's gitlinks count too, and a change to them is read.
    watched.push(git_dir.join("index"));
    let mut paths = submodule_paths(&base, &gitmodules);
    paths.extend(gitlink_paths(&base)?);
    paths.sort();
    paths.dedup();
    for path in paths {
        let checkout = base.join(&path);
        // Watched while missing too, so a submodule initialized later is
        // read then.
        watched.push(checkout.join(".git"));
        if !checkout.join(".git").exists() {
            continue;
        }
        if depth >= max_depth {
            // Too deep to read, so what it configures is unknown: it counts
            // as naming a program rather than as naming none.
            settings.push(ProgramSetting { submodule: Some(path),
                                           key:       UNREAD_SUBMODULE_KEY.to_owned(),
                                           value:     UNREAD_SUBMODULE_VALUE.to_owned(), });
            continue;
        }
        let Some(found) = read_repository(&checkout, depth + 1, max_depth)?
        else {
            continue;
        };
        watched.extend(found.watched);
        settings.extend(found.settings.into_iter().map(|setting| {
                                                      let submodule = match setting.submodule {
                                                          Some(inner) => format!("{path}/{inner}"),
                                                          None => path.clone(),
                                                      };
                                                      ProgramSetting { submodule: Some(submodule),
                                                                       ..setting }
                                                  }));
    }
    Ok(Some(Found { common_dir,
                    settings,
                    watched }))
}

/// The submodule paths `.gitmodules` names, relative to the repository's
/// top. A path that is absolute or climbs out with `..` is not a submodule
/// of this repository and is skipped.
fn submodule_paths(base: &Path, gitmodules: &Path) -> Vec<String> {
    let Some(file) = gitmodules.to_str().filter(|_| gitmodules.is_file())
    else {
        return Vec::new();
    };
    let runner = Runner::new(base).without_trust_check();
    let argv = [CONFIG_FILE_REGEXP_PREFIX,
                &[file, "--get-regexp", SUBMODULE_PATH_PATTERN]].concat();
    // No match is git's exit 1: no submodules.
    let Ok(listing) = runner.run_raw(&argv)
    else {
        return Vec::new();
    };
    listing.split('\0')
           .filter_map(|entry| entry.split_once('\n').map(|(_, path)| path))
           .filter(|path| is_inside(path))
           .map(str::to_owned)
           .collect()
}

/// The paths of the index's gitlinks - submodules, whether `.gitmodules`
/// names them or not - relative to the repository's top. A repository whose
/// index git cannot read has none that `status` would enter.
fn gitlink_paths(base: &Path) -> Result<Vec<String>> {
    let runner = Runner::new(base).without_trust_check();
    let listing = match runner.run_raw(LS_FILES_STAGE) {
        Ok(listing) => listing,
        Err(GitError::Command { .. }) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    // `<mode> <object> <stage>\t<path>`, NUL-ended.
    Ok(listing.split('\0')
              .filter_map(|entry| entry.split_once('\t'))
              .filter(|(meta, _)| meta.split(' ').next() == Some(GITLINK_MODE))
              .map(|(_, path)| path)
              .filter(|path| is_inside(path))
              .map(str::to_owned)
              .collect())
}

/// Whether `path` names somewhere inside the repository: relative, not
/// empty, and not climbing out with `..`.
fn is_inside(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
    && path.is_relative()
    && !path.components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
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
mod tests;

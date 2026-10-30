use std::fs;
use std::path::{Path, PathBuf};

use crate::consts;
use crate::diff::{FileDiff, parse_diff};
use crate::error::{GitError, Result};
use crate::runner::Runner;
use crate::stats::{DiffStats, parse_numstat, untracked_line_count};
use crate::status::{RepoStatus, parse_status};

/// A git repository addressed by working directory. All operations run through
/// a [`Runner`], so they share its timeout and working directory.
#[derive(Debug, Clone)]
pub struct Repository {
    runner: Runner,
}

impl Repository {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { runner: Runner::new(path), }
    }

    pub fn with_runner(runner: Runner) -> Self {
        Self { runner }
    }

    pub fn runner(&self) -> &Runner {
        &self.runner
    }

    /// Every file in the working tree a picker should offer: tracked
    /// files plus untracked ones that are not ignored - submodules' included
    /// (#312).
    ///
    /// Paths are relative to the repository root and NUL-separated on the
    /// wire, so one containing a space, a quote or a newline survives. A
    /// deleted-but-still-tracked file is included - `ls-files` lists the
    /// index, and a caller wanting only what exists on disk should check.
    ///
    /// `ls-files` stops at a submodule, listing only its directory, and its
    /// own `--recurse-submodules` cannot be combined with `--others`, which
    /// would lose a submodule's untracked files. So each submodule is listed
    /// the same way, in its own directory, and its paths prefixed with where
    /// it sits - nested ones by the same recursion. The submodule's own entry
    /// is dropped: it is a directory, not a file. One that is not checked out
    /// is skipped: it has nothing to offer.
    pub fn list_files(&self) -> Result<Vec<String>> {
        let mut files = self.list_own_files()?;
        // Best-effort, like each submodule's own listing below: not being
        // able to find the submodules must not cost the repository's files.
        let submodules = self.submodule_paths().unwrap_or_default();
        files.retain(|path| !submodules.contains(path));
        for submodule in submodules {
            let dir = self.runner.cwd().join(&submodule);
            // Only a checked-out submodule, which has its own `.git` (a file
            // or a directory). One that is not is an empty directory, and git
            // run inside it falls back to the superproject - which reports
            // that same gitlink again, as `./`, and the recursion would list
            // the same directory under an ever longer path.
            if !dir.join(".git").exists() {
                continue;
            }
            let inner = Self::with_runner(self.runner.in_dir(dir));
            // Best-effort per submodule: one git cannot read must not take
            // the rest of the listing with it.
            if let Ok(nested) = inner.list_files() {
                files.extend(nested.into_iter().map(|path| format!("{submodule}/{path}")));
            }
        }
        // During a merge conflict the index holds an entry per stage, so
        // `ls-files` names a conflicted file - or submodule - up to three
        // times. A picker offers each path once. Not `--deduplicate`: that
        // needs git 2.31, and 2.30 is supported.
        files.sort_unstable();
        files.dedup();
        Ok(files)
    }

    /// This repository's own files, stopping at submodules.
    fn list_own_files(&self) -> Result<Vec<String>> {
        let output = self.runner.run_raw(consts::LS_FILES)?;
        Ok(nul_separated(&output))
    }

    /// The paths of the submodules in the index, each once - a conflicted
    /// gitlink has an entry per stage.
    fn submodule_paths(&self) -> Result<Vec<String>> {
        let output = self.runner.run_raw(consts::LS_FILES_STAGE)?;
        let mut paths: Vec<String> =
            nul_separated(&output).into_iter()
                                  .filter_map(|entry| {
                                      // `<mode> <object> <stage>\t<path>`
                                      let (meta, path) = entry.split_once('\t')?;
                                      meta.starts_with(consts::GITLINK_MODE)
                                          .then(|| path.to_owned())
                                  })
                                  .collect();
        paths.sort_unstable();
        paths.dedup();
        Ok(paths)
    }

    /// Parsed `git status --porcelain=v2 --branch`.
    pub fn status(&self) -> Result<RepoStatus> {
        let output = self.runner.run(consts::STATUS)?;
        Ok(parse_status(&output))
    }

    /// Combined line changes: `git diff --numstat` plus `git diff --staged
    /// --numstat`, then each untracked file as one changed file with its line
    /// count as insertions (binary or unreadable untracked files add a file
    /// with no line delta).
    pub fn diff_stats(&self) -> Result<DiffStats> {
        let unstaged = parse_numstat(&self.runner.run(consts::NUMSTAT)?);
        let staged = parse_numstat(&self.runner.run(consts::NUMSTAT_STAGED)?);

        let mut stats = DiffStats::default();
        for (added, deleted, _) in unstaged.iter().chain(&staged) {
            stats.insertions += added;
            stats.deletions += deleted;
            stats.files_changed += 1;
        }

        for entry in self.status()?.untracked() {
            let lines = untracked_line_count(fs::read(self.runner.cwd().join(&entry.path)));
            stats.insertions += lines.unwrap_or(0);
            stats.files_changed += 1;
        }

        Ok(stats)
    }

    /// One path's diff: `git diff --no-color [--staged] -- <path> [<orig>]`,
    /// parsed by [`parse_diff`].
    ///
    /// `staged` selects index-against-HEAD rather than worktree-against-index.
    /// They are different diffs for a path that was staged and then modified
    /// again, which is why the caller chooses.
    ///
    /// `orig_path` is a rename's or copy's source, from
    /// [`FileEntry::orig_path`]. Git detects a rename by comparing both sides,
    /// so a diff scoped to the destination alone reports a whole new file
    /// instead - passing the source back is what keeps a rename a rename.
    ///
    /// `None` when the path has no change on that side.
    ///
    /// [`parse_diff`]: crate::diff::parse_diff
    /// [`FileEntry::orig_path`]: crate::status::FileEntry::orig_path
    pub fn file_diff(&self, path: &str, orig_path: Option<&str>, staged: bool)
                     -> Result<Option<FileDiff>> {
        let mut argv = consts::DIFF.to_vec();
        if staged {
            argv.push(consts::DIFF_STAGED_FLAG);
        }
        argv.push(consts::PATHSPEC_SEP);
        argv.push(path);
        argv.extend(orig_path);

        Ok(parse_diff(&self.runner.run(&argv)?).into_iter().next())
    }

    /// `git add <paths>`. Empty slice is a no-op (no process spawned).
    pub fn stage(&self, paths: &[&str]) -> Result<()> {
        self.run_scoped(consts::ADD, paths)
    }

    /// `git restore --staged <paths>`. Empty slice is a no-op.
    pub fn unstage(&self, paths: &[&str]) -> Result<()> {
        self.run_scoped(consts::RESTORE_STAGED, paths)
    }

    /// `git restore <paths>`. Empty slice is a no-op.
    pub fn discard(&self, paths: &[&str]) -> Result<()> {
        self.run_scoped(consts::RESTORE, paths)
    }

    /// `git add -A`.
    pub fn stage_all(&self) -> Result<()> {
        self.runner.run(consts::ADD_ALL).map(drop)
    }

    /// `git reset HEAD`.
    pub fn unstage_all(&self) -> Result<()> {
        self.runner.run(consts::UNSTAGE_ALL).map(drop)
    }

    /// `git commit -m <message>`.
    pub fn commit(&self, message: &str) -> Result<()> {
        let argv = [consts::COMMIT, &[message]].concat();
        self.runner.run(&argv).map(drop)
    }

    /// Current branch from `git branch --show-current`. `None` when HEAD is
    /// detached (empty output).
    pub fn current_branch(&self) -> Result<Option<String>> {
        let branch = self.runner.run(consts::BRANCH_SHOW_CURRENT)?;
        Ok((!branch.is_empty()).then_some(branch))
    }

    /// Abbreviated commit id of `HEAD` from `git rev-parse --short HEAD` -
    /// what names a detached HEAD, where [`Self::current_branch`] has
    /// nothing.
    pub fn short_head(&self) -> Result<String> {
        self.runner.run(consts::SHORT_HEAD)
    }

    /// Whether the branch has commits its upstream lacks
    /// (`git log @{u}.. --oneline`). `false` when there is no upstream.
    pub fn has_unpushed(&self) -> Result<bool> {
        match self.runner.run(consts::LOG_UNPUSHED) {
            Ok(output) => Ok(!output.is_empty()),
            Err(GitError::Command { .. }) => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// `(ahead, behind)` relative to the upstream from
    /// `git rev-list --left-right --count @{u}...HEAD`. `(0, 0)` when there is
    /// no upstream.
    pub fn ahead_behind(&self) -> Result<(u32, u32)> {
        let output = match self.runner.run(consts::AHEAD_BEHIND) {
            Ok(output) => output,
            Err(GitError::Command { .. }) => return Ok((0, 0)),
            Err(err) => return Err(err),
        };

        // left-right count prints "<behind>\t<ahead>": left = @{u}, right =
        // HEAD.
        let mut counts = output.split_whitespace();
        let behind = counts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let ahead = counts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        Ok((ahead, behind))
    }

    /// The folder's working tree root, from `git rev-parse --show-toplevel`.
    ///
    /// `None` when the folder is not inside a git repository, rather than an
    /// error - a folder an agent works in need not be one, and the caller
    /// (resolving a workspace's repositories) treats that folder as
    /// contributing no repository, not as a failure.
    pub fn toplevel(&self) -> Result<Option<PathBuf>> {
        let output = match self.runner.run(consts::TOPLEVEL) {
            Ok(output) => output,
            Err(GitError::Command { .. }) => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(Some(PathBuf::from(output)))
    }

    /// The directory shared by every worktree of this repository, from
    /// `git rev-parse --git-common-dir`, resolved against the folder and
    /// canonicalized to an absolute, symlink-free path.
    ///
    /// Canonicalizing is what lets two worktrees of one repository compare
    /// equal: git may report the common dir relative to the working
    /// directory, or already absolute, depending on its version and how the
    /// repository was cloned - two different-looking answers for the same
    /// directory otherwise.
    ///
    /// `None` when the folder is not inside a git repository.
    pub fn common_dir(&self) -> Result<Option<PathBuf>> {
        let output = match self.runner.run(consts::GIT_COMMON_DIR) {
            Ok(output) => output,
            Err(GitError::Command { .. }) => return Ok(None),
            Err(err) => return Err(err),
        };
        // `Path::join` with an absolute `output` discards `cwd` and keeps
        // `output`, so this is correct whichever form git chose to report.
        let resolved = self.runner.cwd().join(output);
        Ok(Some(fs::canonicalize(resolved)?))
    }

    /// A named remote's URL, from `git remote get-url <name>`.
    ///
    /// `None` both outside a git repository and when the repository has no
    /// such remote - git fails the command the same way either time, and
    /// neither is an error a caller resolving repositories should see.
    pub fn remote_url(&self, name: &str) -> Result<Option<String>> {
        let argv = [consts::REMOTE_GET_URL, &[name]].concat();
        match self.runner.run(&argv) {
            Ok(url) => Ok(Some(url)),
            Err(GitError::Command { .. }) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Create a worktree at `destination` on a new branch `branch`, via
    /// `git worktree add -b <branch> <destination>` run from this repository.
    /// The runner's error propagates unchanged, so an existing branch name
    /// surfaces as [`GitError::Command`] and no worktree is created.
    pub fn create_worktree(&self, branch: &str, destination: &Path) -> Result<()> {
        let destination = destination.to_str().ok_or_else(|| {
            GitError::Parse("worktree destination path is not valid UTF-8".to_owned())
        })?;
        let argv = [consts::WORKTREE_ADD, &[branch, destination]].concat();
        self.runner.run(&argv).map(drop)
    }

    fn run_scoped(&self, base: &[&str], paths: &[&str]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }

        let argv = [base, paths].concat();
        self.runner.run(&argv).map(drop)
    }
}

/// The non-empty entries of a NUL-separated listing.
fn nul_separated(output: &str) -> Vec<String> {
    output.split('\0')
          .filter(|entry| !entry.is_empty())
          .map(str::to_owned)
          .collect()
}

#[cfg(test)]
mod tests {
    use super::Repository;
    use crate::runner::Runner;

    /// A path-scoped op with no paths must not spawn git. `false` always exits
    /// non-zero, so a spawn would surface as an error.
    #[test]
    fn empty_path_scoped_ops_do_not_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::with_runner(Runner::new(dir.path()).with_program("false"));

        assert!(repo.stage(&[]).is_ok());
        assert!(repo.unstage(&[]).is_ok());
        assert!(repo.discard(&[]).is_ok());
    }
}

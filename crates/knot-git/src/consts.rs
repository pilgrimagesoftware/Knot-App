use std::time::Duration;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How often a running `git` is checked for exit.
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// The least time allowed for `git`'s output to finish draining once it has
/// exited, even past the timeout, so a command that exits a moment before its
/// deadline is not reported as timed out for want of a moment to flush.
pub const OUTPUT_DRAIN_FLOOR: Duration = Duration::from_millis(100);

/// The binary this crate runs, until the application names another through
/// [`crate::program::configure`].
pub const GIT_PROGRAM: &str = "git";

pub const VERSION: &[&str] = &["--version"];

pub const STATUS: &[&str] = &["status", "--porcelain=v2", "--branch"];

/// Every file the repository tracks, plus the untracked ones git would
/// not ignore. `--exclude-standard` is what honours `.gitignore`, the
/// global excludes file and `.git/info/exclude`, so a caller listing files
/// for a picker does not have to reimplement ignore rules.
///
/// `-z` because a path may contain anything but NUL, and without it git
/// C-quotes the awkward ones - which would have to be unquoted to be used.
pub const LS_FILES: &[&str] = &["ls-files",
                                "--cached",
                                "--others",
                                "--exclude-standard",
                                "-z"];

/// The index with each entry's mode, NUL-separated: how [`LS_FILES`]'s caller
/// finds the submodules in a listing, which `ls-files` reports as a single
/// entry - the submodule's directory - with mode [`GITLINK_MODE`].
pub const LS_FILES_STAGE: &[&str] = &["ls-files", "--stage", "-z"];

/// The index mode of a submodule (a "gitlink").
pub const GITLINK_MODE: &str = "160000";

/// Config every git call runs with: no filesystem monitor, which git would
/// otherwise start as a configured program on a plain `status`.
pub const CALL_CONFIG: &[&str] = &["-c", "core.fsmonitor=false"];
/// Config for every call the user did not start
/// ([`CallOrigin::Automated`](crate::runner::CallOrigin)): no hooks.
pub const NO_HOOKS_CONFIG: &[&str] = &["-c", "core.hooksPath=/dev/null"];

/// Diffs Knot reads are git's own: no external diff program, no textconv
/// filter, whatever the repository configures.
pub const DIFF: &[&str] = &["diff", "--no-color", "--no-ext-diff", "--no-textconv"];
pub const DIFF_STAGED_FLAG: &str = "--staged";
/// Separates revisions from paths. Not optional on a path-scoped diff: a path
/// that also names a branch is otherwise ambiguous, and git resolves it as the
/// revision.
pub const PATHSPEC_SEP: &str = "--";

pub const NUMSTAT: &[&str] = &["diff", "--numstat", "--no-ext-diff", "--no-textconv"];
pub const NUMSTAT_STAGED: &[&str] = &["diff",
                                      "--staged",
                                      "--numstat",
                                      "--no-ext-diff",
                                      "--no-textconv"];

pub const ADD: &[&str] = &["add"];
pub const ADD_ALL: &[&str] = &["add", "-A"];
pub const RESTORE_STAGED: &[&str] = &["restore", "--staged"];
pub const UNSTAGE_ALL: &[&str] = &["reset", "HEAD"];
pub const RESTORE: &[&str] = &["restore"];
pub const COMMIT: &[&str] = &["commit", "-m"];

pub const WORKTREE_ADD: &[&str] = &["worktree", "add", "-b"];

/// A folder's working tree root. Absolute already - git resolves it - so
/// unlike [`GIT_COMMON_DIR`] this needs no further canonicalization.
pub const TOPLEVEL: &[&str] = &["rev-parse", "--show-toplevel"];
/// The directory shared by every worktree of one repository. May come back
/// relative to the working directory or already absolute depending on git's
/// version and how the repository was cloned, which is why
/// [`Repository::common_dir`](crate::repository::Repository::common_dir)
/// resolves and canonicalizes it before comparing two worktrees.
pub const GIT_COMMON_DIR: &[&str] = &["rev-parse", "--git-common-dir"];
/// Base argv for a named remote's URL; the remote name is appended.
pub const REMOTE_GET_URL: &[&str] = &["remote", "get-url"];

pub const BRANCH_SHOW_CURRENT: &[&str] = &["branch", "--show-current"];
pub const SHORT_HEAD: &[&str] = &["rev-parse", "--short", "HEAD"];
pub const LOG_UNPUSHED: &[&str] = &["log", "@{u}..", "--oneline"];
pub const AHEAD_BEHIND: &[&str] = &["rev-list", "--left-right", "--count", "@{u}...HEAD"];

/// Where a repository keeps its configuration: the common git folder (its
/// `config`) and this worktree's git folder (its `config.worktree`).
pub const REV_PARSE_GIT_DIRS: &[&str] = &["rev-parse",
                                          "--path-format=absolute",
                                          "--git-common-dir",
                                          "--git-dir"];

/// The worktree's top folder, which `config --show-origin` names files
/// relative to.
pub const REV_PARSE_TOPLEVEL: &[&str] = &["rev-parse", "--path-format=absolute", "--show-toplevel"];

/// Every configuration entry with its scope and the file it came from,
/// following includes, NUL-separated.
pub const CONFIG_LIST_WITH_ORIGIN: &[&str] = &["config",
                                               "--list",
                                               "--show-scope",
                                               "--show-origin",
                                               "--includes",
                                               "-z"];

/// The file naming a repository's submodules, at its top.
pub const GITMODULES_FILE: &str = ".gitmodules";

/// `git config -z --file <file>` - followed by the file and a query - for
/// reading a file such as `.gitmodules` rather than a repository's config.
pub const CONFIG_FILE_REGEXP_PREFIX: &[&str] = &["config", "-z", "--file"];

/// The `.gitmodules` keys holding each submodule's path.
pub const SUBMODULE_PATH_PATTERN: &str = r"^submodule\..*\.path$";

/// How deep repository trust follows submodules inside submodules. Deeper
/// nesting than this is not read.
pub const MAX_SUBMODULE_DEPTH: usize = 8;

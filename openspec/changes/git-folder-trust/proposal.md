## Why

Knot runs git in an agent's folder on its own: status, diffs and stats
whenever a window is open, and worktrees when an agent asks for one. A
repository's own configuration (`.git/config`, `config.worktree`, and files
they include) can name programs that git then runs during those commands. A
filter driver's `clean` runs on `status` and `add`, its `smudge` on
`worktree add`, a `textconv` on a diff, and `hook.<name>.command` even with
`core.hooksPath` set to `/dev/null`. Git has no single option that turns
them all off. Opening a folder in Knot shouldn't run programs the user never
chose to run.

## What Changes

- Before any git call, `knot-git` reads the repository's own configuration
  entries (scopes `local` and `worktree`, following includes, and each
  checked-out submodule's) and keeps the keys that name programs - except
  those Knot already turns off on its own calls (`core.hooksPath`,
  `core.fsmonitor`, `diff.external`, `diff.*.textconv`). The answer is cached per folder until a file it
  was read from changes.
- If the repository names programs and the application's trust policy
  doesn't trust it with exactly those entries, the call runs nothing and
  fails with `GitError::Untrusted`.
- The workspace window shows a banner for the first such repository. It
  names the repository and each configured program, with control characters
  shown visibly, and offers **Trust** and **Not now**. Trust is saved per
  repository with the entries it was given, so a repository whose entries
  change is asked about again. "Not now" stops asking until the next launch;
  the git panel explains why it is empty and offers **Review** to ask again.
- Settings > Coding lists trusted repositories, each with a confirmed
  **Remove trust** action.
- A repository that names no programs, and a folder outside any repository,
  are unaffected. The user's global and system configuration is theirs and
  isn't checked.

## Impact

- `knot-git`: new `trust` module (key matching, detection and cache, policy
  gate), `GitError::Untrusted`, a trust check in `Runner`.
- `knot-core`: `Settings::trusted_repositories` (`TrustedRepository`).
- `knot`: `repository_trust` registry installed at startup, the workspace
  banner, the Settings group, and l10n keys under `repository_trust`.
- Each first git call in a folder, and each one after a configuration file
  changes, runs two to three extra git reads (`rev-parse`, `config --list`).

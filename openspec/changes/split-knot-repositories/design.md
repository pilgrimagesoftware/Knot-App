# Design

## Context

`pilgrimagesoftware/Knot` is public, AGPL-3.0, default branch `develop`, with
`develop` and `main` rulesets requiring the Rust CI matrix. Releases run
through `pilgrimagesoftware/github-actions` reusable workflows, which take the
repository from the Actions context rather than a hard-coded name. The name
is hard-coded in `knot_core::consts::KNOT_REPO` (the bug reporter's target),
`Cargo.toml` `repository`, `crates/knot/Cargo.toml` `homepage`, `README.md`,
several tests, and full URLs in archived OpenSpec changes.

The repository is also a fork of `Kochava-Studios/skwad`. Forking is GitHub
metadata, not git: no clone, mirror or history rewrite changes it. Only leaving
the fork network, or deleting the fork, does.

## Goals / Non-Goals

**Goals:**
- Knot-App outside the Skwad fork network, keeping every issue.
- The shortest possible window in which the app's CI or bug reporter points at
  the wrong repository.
- Each new repository usable from the day it is created: README, license,
  default branch and branch protection matching the app's.

**Non-Goals:**
- Tooling in the meta-repository beyond submodule wiring and a README.

## Decisions

### A new repository and issue transfer, not a rename

A rename keeps the repository a Skwad fork. Instead Knot-App is created fresh
(not a fork), every branch and tag pushed to it, and settings, labels and
rulesets copied as JSON so they diff clean against the old repository. Every
issue moves by `gh issue transfer`, which keeps comments, assignees, state and
labels (once the labels exist in the target), and assigns new numbers. The
old→new numbers are recorded in `knot-app-issue-map.tsv` for rewriting links.

Cost: pull requests and their reviews, releases and Actions history cannot be
transferred and go with the old repository. Releases are rebuilt by running the
release workflow on Knot-App; merge commits still name the pull requests.

Alternative: the **Leave fork network** button in the repository settings,
then the rename. It would keep pull requests and redirects, but GitHub's
documentation warns the detached repository may not keep its issues, and the
action is irreversible. Rejected: open issues must not be lost.

### Order: move, update, verify, delete, then reuse the name

1. Create Knot-App, push, copy settings and transfer issues.
2. Point the primary checkout's `origin` at Knot-App (worktrees share it).
3. Land one PR in Knot-App updating every hard-coded reference, and release it.
   Its CI run proves Actions, rulesets and the reusable release workflows work
   in the new repository.
4. Take the old `Knot` out of the Skwad fork network (**Leave fork network**)
   once the maintainer is satisfied with Knot-App, rather than deleting it.
5. Clear it out - releases, tags, every branch but `main`, its rulesets - and
   replace `main` with the meta-repository's own, unrelated history.

Reusing the repository rather than deleting it keeps its 447 pull requests
and their reviews, which could not be transferred, and leaves no window in
which released builds' bug reports have nowhere to go. Its issues had already
moved to Knot-App, so the leave-network warning about issues no longer
mattered.

Alternative: create the meta-repository under another name (`Knot-Meta`).
Rejected: the request is for the family's front door to be
`pilgrimagesoftware/Knot`.

### Released builds' bug reports

A released build files on `pilgrimagesoftware/Knot`, which is the
meta-repository from step 5 on and exists throughout. The meta-repository keeps issues enabled with a
`config.yml` issue template chooser whose only link sends reporters to
Knot-App's issues, and blank issues disabled - but `gh issue create`, which
the reporter uses, bypasses templates. The meta-repository's README and the
chooser say where app bugs belong, and the maintainer transfers stray issues
with `gh issue transfer`. Cut a release at step 3, so builds that file
on Knot-App replace the old ones quickly.

Alternative: disable issues on the meta-repository. Rejected: `gh issue
create` would fail and the reporter would fall back to the browser compose
URL, which also fails; a transferable issue beats a lost report.

### Submodules use relative URLs and track `develop`

`.gitmodules` in the meta-repository:

```ini
[submodule "Knot-App"]
	path = Knot-App
	url = ../Knot-App.git
	branch = develop
[submodule "Knot-MCP"]
	path = Knot-MCP
	url = ../Knot-MCP.git
	branch = develop
[submodule "Knot-Library"]
	path = Knot-Library
	url = ../Knot-Library.git
	branch = master
```

Relative URLs resolve against the meta-repository's own remote, so an SSH
clone fetches submodules over SSH and an HTTPS clone over HTTPS. Dependabot's
`gitsubmodule` ecosystem opens a weekly PR advancing each pin, so the
meta-repository does not silently fall behind.

### New repositories' defaults

| Repository | License | Default branch | Contents at creation |
| --- | --- | --- | --- |
| Knot (meta) | AGPL-3.0 | `main` | README, LICENSE, `.gitmodules`, issue chooser, dependabot |
| Knot-MCP | MIT | `develop` (git-flow, `master` for releases) | README, LICENSE, AGENTS.md, `openspec/` init |
| Knot-Library | CC BY 4.0 | `master` | README, LICENSE, `personas/.gitkeep` |

Knot-MCP follows the app's git-flow and ruleset shape because it will ship
software; its CI ruleset is added with its first code. It is MIT, not AGPL.
Knot-Library holds prompts and data rather than code, so a content license
suits it better than AGPL, and a single `master` branch is enough. Both
repositories were created with `master`; it is kept rather than renamed.

### Local checkouts stay where they are

The primary checkout keeps its path, `~/Code/ThirdParty/Knot`, so running
agents' folders and the `<checkout>-Worktrees` convention are unaffected; only
`origin`'s URL changes. The meta-repository is cloned separately when wanted.
Docs that say "a checkout at `~/Code/ThirdParty/Knot`" stay correct as
examples.

## Risks / Trade-offs

- [A collaborator's clone silently fetches the meta-repository after step 5]
  → Announce the move; `git fetch` then fails on missing `develop` history
  rather than merging unrelated trees, which makes the problem visible.
- [Bug reports from released builds fail, then land on the meta-repository]
  → Issue chooser, README note, `gh issue transfer`; a release at step 3.
- [Links to old issue numbers break] → Archived OpenSpec issue URLs are
  rewritten from the transfer map. Pull request links keep working: the pull
  requests stay in `Knot`.
- [External links (blog, homepage) break after step 4] → Search
  `pilgrimagesoftware.com` sources for the old URL during step 3.
- [The reusable release workflows or package signing reference the name] →
  Step 3's CI run and the release's `package.yml` run catch this before the
  old repository is cleared.

## Migration Plan

Steps 1-5 above, with Knot-MCP and Knot-Library created before the
submodules are added. Rollback before step 5: transfer issues back to `Knot`
and point `origin` at it again. Leaving the fork network cannot be undone;
after step 5 the old branches, tags and releases are gone from `Knot`, though
every commit survives in Knot-App.

## Open Questions

- Whether Knot-MCP later takes over the app's `knot-mcp` crate or depends on
  it. Decided by the MCP server's own design change.

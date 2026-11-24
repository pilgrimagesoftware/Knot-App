---
name: release-repo
description: Cut a release of Knot - preview the version git-cliff will pick, dispatch the Prepare Release workflow, then follow the release PR, tag, GitHub Release, packaged assets, Homebrew cask and the merge back into develop until each one is confirmed. Use when the user asks to release, cut a release, ship, or publish a new version.
metadata:
  type: engineering
---

# Release this repo

Releases are run entirely by GitHub Actions. Nothing is built, versioned or tagged locally, and
`CHANGELOG.md` is never edited by hand: git-cliff regenerates it from the Conventional Commits
since the last tag. The authoritative description is `CONTRIBUTING.md` (Releases); the
rationale is `docs/adr/0001-git-flow.md`.

The chain, each step triggered by the one before:

1. **Prepare Release** (`prepare-release.yml`, manual dispatch on `develop`) computes the next
   version with `git-cliff --bumped-version`, bumps every crate with `cargo set-version`,
   prepends the changelog, and opens a `release/x.y.z` PR titled `Release x.y.z` against `main`
   with auto-merge (merge commit) enabled.
2. The release PR **auto-merges** into `main` once `main`'s required checks pass. Nobody merges
   it by hand.
3. **Tag Release** (`tag-release.yml`) fires on that merge and tags the merge commit `vX.Y.Z`.
4. **Release** (`release.yml`) fires on the tag push. Its jobs: `release / publish` (workspace
   tests, GitHub Release with git-cliff notes, merge `main` back into `develop`), `package`
   (calls `package.yml`, attaches the assets) and `homebrew` (updates the tap's cask).

Resolve `owner/repo` with `gh repo view --json nameWithOwner --jq .nameWithOwner`; below it is
`<owner>/<repo>`. `<v>` is the version without the `v` (`1.42.0`), the tag is `v<v>`.

## Steps

### 1. Preconditions

- `git fetch --tags origin`, then work from `origin/develop`: that is what gets released.
- CI on the head of `develop` is green:
  `gh run list --workflow ci.yml --branch develop --limit 1 --json conclusion,headSha`.
- No release is already in flight: `gh pr list --base main --state open --json number,headRefName`
  shows no `release/*` PR, and `gh run list --workflow release.yml --limit 1` is not running.
- Do not change files for the release. There is no version-prep commit and no changelog edit.

### 2. Preview and confirm the version

The workflow takes no version input; git-cliff decides from the commit types since the last
tag (`[bump]` in `cliff.toml`):

- any `feat` -> minor bump (`1.41.1` -> `1.42.0`)
- a breaking change (`!` or `BREAKING CHANGE:`) -> major bump
- otherwise (`fix`, `perf`, ...) -> patch bump (`1.41.0` -> `1.41.1`)

Preview it on the same commit the workflow will use:

```bash
git switch --detach origin/develop   # in a worktree, not the primary checkout
git-cliff --bumped-version           # next tag, e.g. v1.42.0
git-cliff --unreleased --bump        # the changelog section it will write
git describe --tags --abbrev=0       # the last tag, for comparison
```

If `--bumped-version` prints the last tag ("There is nothing to bump"), there is nothing to
release: stop and say so. Prepare Release would fail with "Nothing to release" anyway.

If the user named a version (`$ARGUMENTS`) and it differs from the preview, stop and report
both. The only way to change the version is the commit history (for example a `feat` commit
for a minor bump), not a workflow input.

Show the user the version and the changelog preview, and get a yes before dispatching. A
release is public and cannot be taken back by re-running anything.

### 3. Dispatch Prepare Release

```bash
gh workflow run prepare-release.yml --ref develop
sleep 5
RUN=$(gh run list --workflow prepare-release.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch "$RUN" --exit-status
```

### 4. Follow the release PR

```bash
gh pr list --base main --head release/<v> --state all \
  --json number,url,state,autoMergeRequest
gh pr checks <number> --watch
```

`autoMergeRequest` must be set (enabled by `app/psw-ci`). Wait for `state` to become `MERGED`.
If a required check fails, the PR stays open: read the failure (`gh run view <id> --log-failed`)
and report it. Do not merge it by hand or disable auto-merge.

### 5. Confirm the tag

```bash
gh run list --workflow tag-release.yml --limit 1 --json databaseId,conclusion,headBranch
git fetch --tags origin && git rev-parse "v<v>^{commit}"
git rev-parse origin/main   # the tag must point at the release PR's merge commit
```

### 6. Follow the Release run

```bash
RUN=$(gh run list --workflow release.yml --event push --branch v<v> --limit 1 \
  --json databaseId --jq '.[0].databaseId')
gh run watch "$RUN" --exit-status
gh run view "$RUN" --json jobs --jq '.jobs[] | "\(.name): \(.conclusion)"'
```

All of `release / publish`, `package / Package (macOS)`, `package / Archive (Linux)`,
`package / Archive (Windows)` and `homebrew` must succeed. The macOS package takes the longest.

### 7. Check the assets

```bash
gh release view v<v> --json url,isDraft,isPrerelease,assets --jq '.url, (.assets[].name)'
```

Expected, all seven:

- `Knot_<v>_aarch64.dmg`
- `Knot_<v>_aarch64.cdx.json` (CycloneDX SBOM)
- `SHA256SUMS` (covers the DMG and the SBOM)
- `Knot_<v>_x86_64-unknown-linux-gnu.tar.gz` and its `.sha256`
- `Knot_<v>_x86_64-pc-windows-msvc.zip` and its `.sha256`

To check the checksums: `gh release download v<v> --pattern 'Knot_*_aarch64.*' --pattern SHA256SUMS`
into an empty temp directory, then `shasum -a 256 -c SHA256SUMS` there.

### 8. Check the merge-back and the cask

```bash
git fetch origin
git merge-base --is-ancestor origin/main origin/develop && echo "main is merged into develop"
git log -1 --format=%s origin/develop   # chore(release): merge main into develop after v<v>
gh api repos/<owner>/homebrew-tap/contents/Casks/knot-ai.rb --jq .content | base64 -d \
  | grep -E '^\s*(version|sha256)'
```

The cask's `version` must be `<v>` and its `sha256` must match the DMG's line in `SHA256SUMS`.

### 9. Report back

Version and tag, release PR number, the Release URL, the run IDs, the asset list, and
confirmation of the merge-back and the cask. If anything is missing, say which step and why.

## When it fails

- **Prepare Release: "Nothing to release"**: no releasable commits since the last tag (step 2
  should have caught it). Nothing to retry.
- **Release PR sits open**: a required check on `main` failed, or a ruleset blocks the merge.
  Read the check log and report; fixes go to `develop` through a normal PR, then run Prepare
  Release again (it reuses or recreates the `release/<v>` branch).
- **Tag Release didn't run or failed**: the PR was closed without merging, or the PSW CI App
  secrets (`PSW_CI_APP_ID`, `PSW_CI_PRIVATE_KEY`) are missing or the App lost access.
- **A Release job failed**: read `gh run view <id> --log-failed` first. Re-running the failed
  jobs (`gh run rerun <id> --failed`) is fine for a runner fault; a code or config fault needs a
  fix and a new release. `release.yml` also has a `workflow_dispatch` with a `tag` input
  (`gh workflow run release.yml -f tag=v<v>`) for re-running against an existing tag; ask first.
- **Package unsigned**: the Apple secrets aren't set; the DMG still builds with an ad-hoc
  signature. Report it, don't treat it as a failure of the release.
- **Merge-back missing**: `release / publish` failed after publishing. Report it; don't merge
  `main` into `develop` by hand without the user.
- **`gh` TLS/certificate error**: a sandboxed-network restriction, not a GitHub outage. Retry the
  `gh` call outside the sandbox.

Report which step failed and why, not just "the release didn't work."

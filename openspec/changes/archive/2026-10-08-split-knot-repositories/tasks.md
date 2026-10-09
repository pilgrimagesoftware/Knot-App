# Tasks

## 1. Move the app to a standalone repository

- [x] 1.1 Create empty public `pilgrimagesoftware/Knot-App` (not a fork) and push every branch and tag from a bare clone of `pilgrimagesoftware/Knot` (`git push 'refs/heads/*:refs/heads/*' 'refs/tags/*:refs/tags/*'`, not `--mirror`, which trips on read-only `refs/pull/*`); verify `git ls-remote --heads --tags` is identical on both and `gh repo view pilgrimagesoftware/Knot-App --json isFork` is `false`
- [x] 1.2 Copy repository settings, labels (`gh label clone`) and the `develop`/`main` rulesets, and enable the security features (Dependabot alerts and updates, secret scanning and push protection, private vulnerability reporting, code scanning default setup); squash merge is disabled to match the rulesets' merge methods; verify the settings, label and ruleset JSON diff clean against the old repository
- [x] 1.3 Transfer every issue, open and closed, oldest first with `gh issue transfer`, recording old→new numbers in `knot-app-issue-map.tsv` beside the checkout; verify the old repository lists no issues and Knot-App holds 43 open and 154 closed
- [x] 1.4 Set the primary checkout's `origin` to `git@github.com:pilgrimagesoftware/Knot-App.git`; verify `git fetch origin` succeeds and `develop` matches `origin/develop`
- [x] 1.5 After 2.5, take `pilgrimagesoftware/Knot` out of the Skwad fork network with **Leave fork network** (maintainer) instead of deleting it; verify `gh repo view pilgrimagesoftware/Knot --json isFork` is `false`

## 2. Update references in Knot-App

- [x] 2.1 Change `KNOT_REPO` in `crates/knot-core/src/consts.rs` to `pilgrimagesoftware/Knot-App` and update the tests and fixtures in `crates/knot`, `crates/knot-forge` that name the repository; verify `make test`
- [x] 2.2 Update `repository` in `Cargo.toml`, `homepage` in `crates/knot/Cargo.toml`, the README badges and clone command, and repository names in `AGENTS.md`, `CONTRIBUTING.md` and `.claude/skills/project-*`; verify `grep -rn "pilgrimagesoftware/Knot\b"` outside `target/` finds only intended meta-repository references
- [x] 2.3 Rewrite full `github.com/pilgrimagesoftware/Knot/issues/N` URLs in `openspec/changes/archive/` to their new Knot-App numbers from `knot-app-issue-map.tsv`; `/pull/N` URLs have no successor and stay as they are; verify the grep finds only `/pull/` URLs there
- [x] 2.4 Check the `pilgrimagesoftware/github-actions` reusable workflows and `.github/workflows/package.yml` for the literal repository name; verify by a green CI run of the PR and a `package.yml` run on it
- [x] 2.5 Open, merge (merge commit) and release the reference PR through the git-flow release workflows; verify a GitHub Release exists on Knot-App whose build files a test bug report on Knot-App (then close that issue)

## 3. Create Knot-MCP and Knot-Library

- [x] 3.1 Create public `pilgrimagesoftware/Knot-MCP` with an MIT LICENSE; create `develop` from `master` as the default branch; verify `gh repo view` shows it public, MIT, default branch `develop`
- [x] 3.2 Add to Knot-MCP a README describing the cross-Knot, multi-user MCP server as planned, AGENTS.md, and `openspec init`, on `develop`
- [x] 3.3 Create public `pilgrimagesoftware/Knot-Library` with a CC BY 4.0 `LICENSE.md` on `master`; verify `gh repo view` shows license `CC-BY-4.0` and default branch `master`
- [x] 3.4 Add to Knot-Library a README describing it as the home of importable personas and shared data (format to be defined), and `personas/.gitkeep`; verify `master` holds them

## 4. Create the meta-repository

- [x] 4.1 Confirm sections 1-2 are done and every commit on every branch and tag of `pilgrimagesoftware/Knot` exists in Knot-App; record the old refs in `old-knot-refs.txt` beside the checkout, then delete its releases, tags, branches other than `main`, and its `develop`/`main` rulesets; verify `git ls-remote` lists only `main` and its pull requests remain
- [x] 4.2 Replace `main` of `pilgrimagesoftware/Knot` with a fresh, unrelated history (default branch `main`, AGPL-3.0) holding a README introducing the three repositories and `git clone --recurse-submodules`; verify `gh repo view pilgrimagesoftware/Knot --json defaultBranchRef` is `main`
- [x] 4.3 Add the three submodules with the relative URLs and branches in design.md, and commit; verify a fresh `git clone --recurse-submodules` over HTTPS and over SSH checks out all three
- [x] 4.4 Add `.github/ISSUE_TEMPLATE/config.yml` (blank issues disabled, links to Knot-App, Knot-MCP and Knot-Library issues) and a `gitsubmodule` dependabot config; verify the issue chooser on GitHub shows the Knot-App link
- [x] 4.5 Protect the meta-repository's `main` (pull request required, no force-push); verify with `gh api repos/pilgrimagesoftware/Knot/rulesets`

## 5. Announce

- [x] 5.1 Broadcast the move to the knot and update the repository description and homepage links on Knot-App and pilgrimagesoftware.com; verify the homepage links resolve to Knot-App

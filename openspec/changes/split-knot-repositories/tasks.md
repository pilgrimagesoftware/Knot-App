# Tasks

## 1. Move the app to a standalone repository

- [x] 1.1 Create empty public `pilgrimagesoftware/Knot-App` (not a fork) and push every branch and tag from a bare clone of `pilgrimagesoftware/Knot` (`git push 'refs/heads/*:refs/heads/*' 'refs/tags/*:refs/tags/*'`, not `--mirror`, which trips on read-only `refs/pull/*`); verify `git ls-remote --heads --tags` is identical on both and `gh repo view pilgrimagesoftware/Knot-App --json isFork` is `false`
- [x] 1.2 Copy repository settings, labels (`gh label clone`) and the `develop`/`main` rulesets, and enable the security features (Dependabot alerts and updates, secret scanning and push protection, private vulnerability reporting, code scanning default setup); squash merge is disabled to match the rulesets' merge methods; verify the settings, label and ruleset JSON diff clean against the old repository
- [x] 1.3 Transfer every issue, open and closed, oldest first with `gh issue transfer`, recording old→new numbers in `knot-app-issue-map.tsv` beside the checkout; verify the old repository lists no issues and Knot-App holds 43 open and 154 closed
- [x] 1.4 Set the primary checkout's `origin` to `git@github.com:pilgrimagesoftware/Knot-App.git`; verify `git fetch origin` succeeds and `develop` matches `origin/develop`
- [ ] 1.5 After 2.5, delete `pilgrimagesoftware/Knot` (maintainer, once satisfied with Knot-App); verify `gh repo view pilgrimagesoftware/Knot` fails

## 2. Update references in Knot-App

- [x] 2.1 Change `KNOT_REPO` in `crates/knot-core/src/consts.rs` to `pilgrimagesoftware/Knot-App` and update the tests and fixtures in `crates/knot`, `crates/knot-forge` that name the repository; verify `make test`
- [x] 2.2 Update `repository` in `Cargo.toml`, `homepage` in `crates/knot/Cargo.toml`, the README badges and clone command, and repository names in `AGENTS.md`, `CONTRIBUTING.md` and `.claude/skills/project-*`; verify `grep -rn "pilgrimagesoftware/Knot\b"` outside `target/` finds only intended meta-repository references
- [x] 2.3 Rewrite full `github.com/pilgrimagesoftware/Knot/issues/N` URLs in `openspec/changes/archive/` to their new Knot-App numbers from `knot-app-issue-map.tsv`; `/pull/N` URLs have no successor and stay as they are; verify the grep finds only `/pull/` URLs there
- [x] 2.4 Check the `pilgrimagesoftware/github-actions` reusable workflows and `.github/workflows/package.yml` for the literal repository name; verify by a green CI run of the PR and a `package.yml` run on it
- [x] 2.5 Open, merge (merge commit) and release the reference PR through the git-flow release workflows; verify a GitHub Release exists on Knot-App whose build files a test bug report on Knot-App (then close that issue)

## 3. Create Knot-MCP and Knot-Library

- [x] 3.1 Create public `pilgrimagesoftware/Knot-MCP` with an MIT LICENSE; create `develop` from `master` as the default branch; verify `gh repo view` shows it public, MIT, default branch `develop`
- [ ] 3.2 Add to Knot-MCP a README describing the cross-Knot, multi-user MCP server as planned, AGENTS.md, and `openspec init`, on `develop`
- [x] 3.3 Create public `pilgrimagesoftware/Knot-Library` with a CC BY 4.0 `LICENSE.md` on `master`; verify `gh repo view` shows license `CC-BY-4.0` and default branch `master`
- [ ] 3.4 Add to Knot-Library a README describing it as the home of importable personas and shared data (format to be defined), and `personas/.gitkeep`; verify `master` holds them

## 4. Create the meta-repository

- [ ] 4.1 Confirm steps 1-3 are done, the Knot-App release is out and the old repository is deleted (1.5); verify `gh repo view pilgrimagesoftware/Knot` fails, so the name is free
- [ ] 4.2 Create public `pilgrimagesoftware/Knot` (default branch `main`, AGPL-3.0) with a README introducing the three repositories and `git clone --recurse-submodules`; verify `gh repo view pilgrimagesoftware/Knot --json name` returns `Knot`, not `Knot-App`
- [ ] 4.3 Add the three submodules with the relative URLs and branches in design.md, and commit; verify a fresh `git clone --recurse-submodules` over HTTPS and over SSH checks out all three
- [ ] 4.4 Add `.github/ISSUE_TEMPLATE/config.yml` (blank issues disabled, link to Knot-App issues) and a `gitsubmodule` dependabot config; verify the issue chooser on GitHub shows the Knot-App link
- [ ] 4.5 Protect the meta-repository's `main` (pull request required, no force-push); verify with `gh api repos/pilgrimagesoftware/Knot/rulesets`

## 5. Announce

- [ ] 5.1 Broadcast the move to the knot and update the repository description and homepage links on Knot-App and pilgrimagesoftware.com; verify the homepage links resolve to Knot-App

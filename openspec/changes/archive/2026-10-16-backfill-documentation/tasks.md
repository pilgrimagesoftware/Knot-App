# Tasks

## 1. Navigation and structure

- [x] 1.1 Add per-version `nav` entries to `docs/site/_data/versions.yml` for the latest tree and the v0.1 snapshot, then verify `bundle exec jekyll build` renders each entry.
- [x] 1.2 Render the selected version's navigation from the data file in `docs/site/_layouts/default.html`, then verify the sidebar, version selector, and page bodies agree on the current version.
- [x] 1.3 Rewrite `docs/site/latest/index.md` as a landing page that links every guide and reference page, then verify no link in the page is a dead path.

## 2. Guides

- [x] 2.1 Write `docs/site/latest/workspaces.md` from `workspace-manager-ui`, `dashboard`, `window-lifecycle`, and `quit-warning`, then verify the described actions match those contracts.
- [x] 2.2 Write `docs/site/latest/agents.md` from `agent-editor-ui`, `agent-launch-command`, `agent-lifecycle`, `agent-list-ui`, `activity-detection`, `working-indicator`, `acp-panel-ui`, `agent-subagents`, and `agent-processes`, then verify the agent lifecycle and panel behavior match those contracts.
- [x] 2.3 Write `docs/site/latest/coordination.md` from `mcp-server`, `mcp-tools`, `mcp-messaging`, `agent-registry`, `task-graph`, and `agent-hooks`, then verify the tool names and routing rules match those contracts.
- [x] 2.4 Write `docs/site/latest/git-worktrees.md` from `git-panel-ui`, `git-operations`, `worktree-management`, `repo-discovery`, and `pull-request-tracking`, then verify the described Git and worktree actions match those contracts.
- [x] 2.5 Write `docs/site/latest/settings.md` from `settings-ui`, `settings-persistence`, `appearance-mode`, `desktop-notifications`, `data-import`, `import-ui`, `bug-reporting`, `about-ui`, and `app-menu`, then verify the described settings and windows match those contracts.
- [x] 2.6 Write `docs/site/latest/personas-prompts.md` from `personas`, `prompt-library`, and `library-windows`, then verify the persona and prompt lifecycles match those contracts.

## 3. Reference

- [x] 3.1 Move `docs/agent-cli-arguments.md` to `docs/site/latest/reference/agent-cli-arguments.md` with site front matter, then verify the argument tables match `agent-launch-command`.
- [x] 3.2 Move `docs/agent-shells.md` to `docs/site/latest/reference/agent-shells.md` with site front matter, then verify `KNOT_AGENT` guidance is unchanged.
- [x] 3.3 Write `docs/site/latest/reference/keybindings.md` from the `keybindings` contract, then verify every listed binding matches it.

## 4. Validation

- [x] 4.1 Build the site locally with `bundle exec jekyll build` from `docs/site`, then verify the build completes with no Liquid or Markdown errors.
- [x] 4.2 Open every path listed in `_data/versions.yml` in the generated output, then verify no navigation target is missing.
- [x] 4.3 Repoint the README agent-shells link and any other reference at the moved pages, then verify no remaining link names the old `docs/*.md` path.

## Workflow follow-up

- Archive the change after the repository's review requirements are satisfied.
- Verify the archived result.

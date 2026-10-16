## Why

The published documentation site ships with only an Overview and an
Installation page. Every other capability the Rust port implements - workspaces,
the agent editor and agent list, coordination over MCP, Git and worktrees,
settings, personas, and the prompt library - has no user-facing documentation,
so a reader of the site cannot learn what the application does or how to use it.
`AGENTS.md` requires documentation to be written for features and kept up to
date, and the two substantive documents that do exist (`docs/agent-shells.md`,
`docs/agent-cli-arguments.md`) are not reachable from the site at all.

## What Changes

- Add user-facing guide pages under `docs/site/latest/` for workspaces, agents,
  coordination, Git and worktrees, settings, and personas and prompts.
- Add reference pages under `docs/site/latest/reference/` for the agent command
  lines, agent shells, and keybindings, folding the loose `docs/*.md` documents
  into the site.
- Make the site sidebar navigation data-driven from `_data/versions.yml`, so a
  version declares the pages it carries and adding a page does not require
  editing the layout.
- Link the new pages from the landing page and keep every page versioned and
  reachable from the version selector.

## Capabilities

### New Capabilities

- `documentation-content`: the set of user-facing pages the published site must
  carry, what each page must explain, how pages are addressed and linked, and
  how navigation is derived from version data.

### Modified Capabilities

None.

## Impact

- `docs/site/latest/` gains guide and reference pages.
- `docs/site/_data/versions.yml` gains per-version navigation entries.
- `docs/site/_layouts/default.html` renders navigation from that data.
- `docs/site/latest/index.md` points at the new pages.
- No application code changes; the pages describe behavior pinned by the
  existing `openspec/specs/` contracts.

# Design

## Context

See `proposal.md` for motivation and the `documentation-content` delta for
behavior. The Jekyll site, version manifest, shared layout, and authoring rules
already exist from the archived `docs-github-pages` change. What is missing is
content: the `latest/` tree carries an overview and an installation page only,
and the sidebar in `_layouts/default.html` hard-codes those two links.

The behavior each page must describe is already pinned by contracts under
`openspec/specs/`. The pages are a reader-facing restatement of those specs, not
a second source of truth: where a page and a spec disagree, the spec wins.

## Goals / Non-Goals

**Goals:**

- Give the site a page for each user-facing capability area, written as tasks a
  reader performs rather than a restatement of internal requirements.
- Address every page with a stable `/latest/<page>/` permalink and list it in
  the version's navigation data.
- Fold the two loose documents under `docs/` into the site so there is one
  documentation home.

**Non-Goals:**

- Generating pages from `openspec/specs/` automatically. The specs are written
  for implementers and change too often to publish verbatim.
- Documenting internal crate structure, or anything a person using the packaged
  application cannot observe.
- Adding a page for every spec. Closely related specs are grouped under one
  reader-facing page.
- Backfilling historical version snapshots beyond `v0.1`, which stays a
  landing-page-only release snapshot.

## Decisions

### Group capability specs into reader-facing pages

The ~50 capability specs are grouped into six guides and three reference pages
by the task a reader is trying to accomplish:

| Page | Groups |
|---|---|
| `workspaces` | workspace manager, dashboard, window lifecycle, quit warning |
| `agents` | agent editor, launch command, lifecycle, agent list, activity, panel UI, subagents, processes |
| `coordination` | MCP server, MCP tools, messaging, agent registry, task graph, hooks |
| `git-worktrees` | Git panel, Git operations, worktree management, discovery, pull-request tracking |
| `settings` | settings UI and persistence, appearance, notifications, import, bug reporting, about, app menu |
| `personas-prompts` | personas, prompt library, library windows |
| `reference/agent-cli-arguments` | agent launch command per supported CLI |
| `reference/agent-shells` | shell-agent environment and prompt-theme guidance |
| `reference/keybindings` | keybindings |

One page per spec was rejected: it produces dozens of short pages that duplicate
the specs and make the site hard to read.

### Derive navigation from version data

`_data/versions.yml` grows a `nav` list per version, each entry a title and a
version-relative path. `_layouts/default.html` renders the selected version's
entries, prefixed with that version's path. This keeps the version selector and
the sidebar consistent and means a new page is announced in the data file rather
than in the layout.

A separate `_data/nav.yml` was considered and rejected: the authoring guide
already names `_data/versions.yml` as the navigation source, and navigation is
inherently per-version.

### Move the loose documents into the site

`docs/agent-shells.md` and `docs/agent-cli-arguments.md` become
`docs/site/latest/reference/agent-shells.md` and
`docs/site/latest/reference/agent-cli-arguments.md`. Their content is preserved
and given the site's front matter and links. The originals are removed so there
is one home; the README link to `docs/agent-shells.md` is repointed at the site
page.

## Risks / Trade-offs

- [Pages drift from the specs as the port continues] -> Each page names the
  capability specs it summarizes in a closing note, so a spec change points at
  the page that needs updating.
- [Navigation data and the page tree can disagree] -> The build is validated by
  opening every path listed in `versions.yml`, as the authoring guide requires.
- [A guide grows past what a reader can scan] -> Group by task and link to
  reference pages instead of inlining every option.

## Migration Plan

1. Add the `nav` entries to `_data/versions.yml` and render navigation from the
   layout.
2. Add the guide and reference pages under `latest/`.
3. Point the landing page and README at the new pages.
4. Build the site locally and open every path in `versions.yml`.
5. Roll back by reverting the change commit; the previous site is a static tree.

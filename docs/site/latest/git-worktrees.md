---
title: Git and worktrees
version: latest
permalink: /latest/git-worktrees/
---

# Git and worktrees

Knot discovers the repositories under a source folder, lets you review and commit
an [agent]({{ '/latest/agents/' | relative_url }})'s work in a Git panel, creates
worktrees on new branches, and tracks the pull requests your agents open.

## How repositories and worktrees are found

Knot scans the immediate children of the source folder with filesystem reads
only, never invoking `git`. A child whose `.git` is a directory is a repository,
its branch name read from `.git/HEAD` when that holds `ref: refs/heads/<name>`,
and otherwise from the folder name (as on a detached HEAD). A child whose `.git`
is a file is a linked worktree, resolved to its owning repository through the
`gitdir` line pointing at `.git/worktrees/...`.

A linked worktree's name is its folder name with a leading `<repo-name>-` prefix
stripped, so `app-hotfix` under repository `app` shows as `hotfix`. Repositories
are listed case-insensitively by name, the primary clone ahead of its linked
worktrees. A missing or non-directory source folder gives an empty result and no
watch; changing the source folder rescans once after about a second.

## The Git panel

The panel is scoped to one agent's folder, appears beside that agent's content
in the same window, creates no separate window, and closes back to what the
window showed. A folder that is not a working tree is reported as not a
repository rather than shown as clean.

The panel shows one of four states in order: loading, error, clean working tree,
or working tree with changes, with loading only before any status has loaded and
a refresh keeping the content. A failed read is an error, not a clean tree. On a
branch the header shows the branch name and ahead and behind counts against its
upstream, each only when above zero; a detached HEAD shows no branch name.

### Working tree sections

The working tree is shown as up to four labelled sections in order — staged,
unstaged, untracked, conflicts — each omitted when empty and each showing its
entry count, with the list scrolling as one region. They are independent filters
over the same status, not a partition, so a path with both a staged and an
unstaged change appears in each as two separately selectable rows.

A row's actions are stage when not staged, unstage when staged, and discard when
neither staged nor untracked; no action deletes an untracked file. Discard asks
for confirmation naming the path, while staging, unstaging and their bulk forms
do not. The staged section offers unstage-all and the unstaged section stage-all
— `git add -A`, including untracked files — and the untracked and conflict
sections offer no bulk action.

### Diffs

Selecting a row shows its diff below the list; with no selection the pane invites
you to select a file, and otherwise shows the path and the addition and deletion
counts, each only above zero, over the unified diff. The staged side shows the
index-against-HEAD diff, every other side the working-tree diff. Lines are
monospaced and marked as addition, deletion, context or hunk header. A binary
file is reported as binary, a diff with no hunks reports no changes, and a large
diff renders a stated, bounded prefix.

### Staging and committing

A commit control is offered only while something is staged, opening a multi-line
message prompt with confirm and cancel; confirm is unavailable while the message
is empty or only whitespace and while a commit is in flight. The message is
trimmed of surrounding whitespace with internal blank lines kept, so the
subject-blank-line-body convention survives. On success the prompt closes and the
panel refreshes; on failure it stays, shows the failure and keeps the message.
The panel reloads the status after any staging action succeeds or when the tree
changes, surfaces a failed action, and keeps its own writes from re-triggering.

## Creating a worktree

Creating a worktree runs `git worktree add -b <branch> <destination>` from the
source repository and creates the branch as part of it; an existing branch name
fails with no worktree created. The suggested destination is a sibling directory
named `<repo-name>-<sanitized-branch>`, sanitizing by replacing `/` and space
with `-`: repository `/src/app` with branch `feat/login` suggests
`/src/app-feat-login`, beside the repository rather than inside it.

## Pull request tracking

Knot watches each agent's output for GitHub pull request URLs and records every
distinct one against the agent whose output carried it and that agent's
workspace. A URL of the form `https://<host>/<owner>/<repo>/pull/<number>` on
`github.com` or a GitHub Enterprise host is recorded with any trailing path or
query stripped, so the same pull request linked twice is recorded once, and a
URL that does not match is not recorded. Both a panel-protocol agent's tool-call
text and a terminal agent's rendered text are watched. Recording is idempotent
per agent — seeing a URL again adds no record and keeps the first-seen time —
and two agents printing the same pull request produce one record each.

Records persist across a restart, so a workspace's list is the set its agents
have opened, not just those since launch, and each is kept until you remove it,
its agent or workspace is removed, or it expires. A record carries the URL, its
agent and workspace, and the first-seen time, but no fetched state, so a restart
shows rows as pending until state is fetched.

Knot fetches each record's number, title, draft/open/merged/closed state and,
while draft or open, its checks summary; a merged or closed row shows no checks,
and an open or draft row is coloured by whether it can land and, if not, why
(blocked, behind its base, or checks running). A row fetched as merged longer
than the 24-hour retention window ago is dropped, and only merged pull requests
expire. When state cannot be fetched the records are still listed and open in a
browser, with one message stating why; a pull request the forge does not know is
shown as not found.

### The Changes view

The workspace window offers a Changes view, reached from a launcher row labelled
Changes in the agent sidebar and shown in place of the window's content. Its tab
bar has three tabs in order — Pull Requests, Issues and OpenSpec — each keeping
its own search, filters and sort; the chosen tab is held per window, pull request
state is fetched only while the Pull Requests tab is shown, and a relaunched
window opens on Pull Requests.

The Pull Requests tab lists the pull requests recorded against the workspace's
agents, grouped by the agent that opened them and ordered by the sort you choose,
newest first until you choose another. A pull request is listed once however many
agents recorded it, in a group headed by all of them, and shared groups come
before single-agent groups. The launcher row counts pull requests only, each
once, as open, merged and closed.

### Working with the list

Clicking a row opens its URL in your default browser, never an embedded view, and
a row is clickable even without state; if the URL cannot be opened the failure is
reported without removing the record or closing the view. A row can be removed,
affecting only Knot's record with nothing changed on GitHub, and the removal
persists across a restart, recorded again only if an agent prints its URL again.

The toolbar above the groups holds the search field, the status and agent
filters, the sort order and the list actions. Search matches a row's title,
number (`#<number>`), repository (`<owner>/<repo>`), URL or opening agent's name,
ignoring case; status toggles cover open, merged, closed, not found and pending
with a count each; and the sort picker offers newest first, oldest first, needs
attention and repository.

## Source contracts

- openspec/specs/repo-discovery/spec.md
- openspec/specs/worktree-management/spec.md
- openspec/specs/git-operations/spec.md
- openspec/specs/git-panel-ui/spec.md
- openspec/specs/pull-request-tracking/spec.md

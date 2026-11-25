---
title: Layouts
version: latest
permalink: /latest/layouts/
---

# Layouts

A workspace window can show up to four agents at once. Each agent sits in its
own pane, a terminal for a Terminal-mode agent and a conversation panel for a
Panel-mode one, under a header with the agent's name. The layout belongs to the
workspace, so two workspaces can use different ones, and Knot restores it when
the window reopens.

## The Layout menu

The Layout menu in the window's content header appears when the workspace
has two or more agents and the agent view is showing. Its button is an icon of
the current layout; hover it to see the layout's name. The menu lists each
layout with its icon:

- **Single**: one pane.
- **Split Vertical**: two panes side by side.
- **Split Horizontal**: two panes stacked.
- **Three Panes**: one tall pane on the left and two stacked on the right. It
  needs three agents.
- **Four Panes**: a two-by-two grid. It needs four agents.

The current mode is checked. Choosing a mode keeps the agents already in panes,
in order. New panes fill with the workspace's other agents in sidebar order,
and agents past the new pane count leave the layout. **Single** keeps only the
focused pane's agent.

### Panes start their agents

Putting an agent in a pane starts it if it isn't running, whether you got there
from the Layout menu, a sidebar click, a number shortcut, or by creating a
companion. A layout that Knot restores when the window opens doesn't start
anything: a passive agent's pane shows the stopped view until you start it.

## Resize the panes

Drag a divider between two panes to resize them. The pane moves with the
pointer, stops at 25% and 75%, and the ratio is saved when you release, so it
is there after a restart. Three Panes has two dividers: the vertical one, and a
horizontal one that spans the right column only. In Four Panes the vertical
and the horizontal divider each run the full width or height, so dragging one
moves both columns or both rows together. Switching modes keeps the ratios, so
switching away and back finds the dividers where you left them.

While you drag, a terminal keeps its rows and columns: a pane that grows shows
empty space at its edge, and one that shrinks clips its text. When you release,
each terminal is resized once, so a full-screen program such as `vim` or `htop`
redraws once rather than on every step of the drag. A conversation panel
reflows as you drag.

## Focus and dimming

One pane is focused. Its agent is the window's selected agent: the sidebar
highlights it, the menus and keyboard shortcuts act on it, and the git and
artifact panels show it. Every other pane is dimmed. Click in a pane to focus
it, and typing goes to the focused pane.

## Choosing agents in the sidebar

Clicking an agent row uses the first of these rules that applies:

1. The agent has companions, the left-most pane is focused, and you aren't
   holding **⌘** (Ctrl on other platforms): Knot shows the agent and up to three
   of its companions, with the agent focused.
2. The agent is already in a pane: that pane takes focus.
3. The left-most pane's agent has companions: the clicked agent is shown alone.
4. Otherwise the clicked agent replaces the focused pane's agent.

**⌘-click** skips only the first rule, so you can put an owner into a pane
without pulling in its companions. The number shortcuts (⌘1 to ⌘9) follow the
same rules.

## Companions and removal

A new companion opens beside its owner and takes focus, whether you made it
with **New Shell Companion**, **New Companion…**, or an agent made it over MCP.
A single layout becomes a split, a split becomes Three Panes, and Three Panes
becomes Four Panes. In a full grid the companion replaces the last pane that
isn't its owner's.

If the owner isn't in a pane, a companion you create gives an owner and
companion split. One an agent creates leaves your layout alone and appears in
the sidebar only.

Removing an agent that has a pane, or a shell companion's process exiting,
removes its pane and shrinks the layout to fit: Four Panes with three agents
left becomes Three Panes, two left a split, one left Single. Focus stays on the
agent that had it. If that agent was the one removed, focus stays at the same
position, or on the last pane when that position no longer exists.

## Example

A workspace has the agents `api`, `web`, `tests` and `docs`, with `web` on
screen.

1. Choose **Four Panes**. `web` keeps pane 0, and `api`, `tests` and `docs`
   fill the others in sidebar order and start if they weren't running.
2. Drag the vertical divider to give the two left panes 60% of the width. The
   ratio is saved when you let go.
3. Click `tests` in the sidebar. It is already shown, so its pane takes focus.
4. Open **New Shell Companion** on `tests`. The shell replaces the last pane
   that isn't `tests`', which is `docs`, and takes focus.
5. Remove the shell, or let it exit. Its pane goes, the layout drops to Three
   Panes, and focus lands on the last pane, `tests`.

## Source contracts

- openspec/specs/workspace-layouts/spec.md
- openspec/specs/agent-list-ui/spec.md

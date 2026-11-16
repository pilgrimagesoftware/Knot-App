---
title: Workspaces
version: latest
permalink: /latest/workspaces/
---

# Workspaces

A workspace groups AI coding agents that share a folder and a working context.
Each has a window with a terminal view and a dashboard view; the
[Agents]({{ '/latest/agents/' | relative_url }}) page covers its agent rows.
This page covers creating and opening workspaces, the dashboard, and quitting.

## The Workspaces window

The Workspaces window lists every workspace you have created, one row each.

### Create and rename a workspace

Open the new-workspace dialog from the Workspaces window, type a name, then
press Return or click Create. Renaming uses the same dialog in its other mode:
edit the name, then press Return or click Save. Leading and trailing spaces are
trimmed, so `  backend  ` is stored as `backend`.

Create (and Save when renaming) is disabled while the name is empty or
whitespace-only. Pressing Return then does nothing: the dialog stays open, the
name field stays focused, and no error appears. Escape closes the dialog,
discards what you typed and clears any validation error. Return and Escape work
while the name field has focus, where focus starts.

When the workspace rows do not all fit in the window, scroll the list
vertically; the last workspace is reachable by scrolling. When every row fits,
the list shows them all without a scrollbar.

## Open a workspace window

Opening a workspace gives you its window. A workspace has at most one window.

- Opening one that is already open brings its window to the front and focuses
  it, from the Workspaces window or a Command Center card alike; no second
  window appears.
- Activating an open window keeps its state: sidebar width, scroll positions,
  running sessions, and its view mode are not reset.
- A request that names an agent (a card click does) selects it in the window.
- Closing a workspace window returns it to having no window, so opening it
  again opens a window. Different workspaces open different windows.

## The dashboard

### The Dashboard row

Each workspace window shows a labelled **Dashboard** row at the top of its
agent list, above the first agent row and shaped like them. Clicking it
switches that window between its terminal view and its dashboard view without
opening another window, and the row has a selected background while the
dashboard shows. Clicking a card, or the row again, returns to the terminal.

### Agent cards

Each card shows an agent's avatar, name, status text and color, folder name,
and git diff counts. A Running agent shows the orange status indicator with its
current status text; a folder with uncommitted changes shows insertions,
deletions and file counts. Sort cards by manual order, name, or status.

A card shows an agent as **not running** when it has no session Knot can send
to: it hasn't started, it was deactivated, its workspace window is closed, or
its connection to the agent failed.

### Status summary

Beside the workspace's name, a summary counts its agents by state - for
example `● 1 awaiting input  ● 2 working  ● 3 idle` - in the order that needs
you first: awaiting input, working, idle, error, then not running. Each dot is
the color the cards use for that state. Shell agents aren't counted.

### Quick prompt

Every card except a shell agent's has a one-line field at the bottom. Type a
prompt and press **Return** to send it to that agent without opening it, or
**Cmd+Return** to send it and open the agent. It goes the same way a prompt
from the agent's own message box does: straight away if the agent is free, or
into its queue if it's working, waiting on you, or held by a usage limit - the
field says **Will queue until it can take it** when that's the case. The field
clears once the prompt is taken, and is disabled while the agent isn't
running.

A workspace with no agents to show displays **No agents** instead of a card
grid. Every other workspace group has an **Add Agent** tile, which opens the
agent-creation dialog prefilled with that workspace's folder, if an agent
there supplies one.

## The Command Center

The Command Center is a separate window showing the same card grid for every
attached workspace at once, each workspace as its own section. Open it from the
Window menu (or its key equivalent) or from the workspace manager's toolbar.

- It is a single window: opening it again brings the existing one to the front,
  and closing it and opening it again opens a fresh one.
- Its content scrolls when the sections and cards overflow, so every workspace
  and card stays reachable; cards are not shrunk to fit, and a short grid does
  not scroll.
- Its header shows the status summary over every workspace's agents, and each
  section shows its own beside its name.
- A card's quick prompt goes to the agent through its workspace window. With
  that window closed the agent has no session, so the field is disabled.
- Clicking an agent card opens that agent's workspace window (or focuses it) in
  its terminal view with that agent selected; clicking a workspace heading
  opens the workspace without selecting an agent.

## Where a restored window opens

A workspace window remembers its position and size, and reopens on a currently
attached display with its title bar visible and grabbable.

- If the remembered display is still attached, the window opens at exactly its
  remembered position and size.
- If the remembered bounds lie partly off every attached display, the window
  keeps its size and moves just enough to bring its title bar on screen.
- If the bounds are entirely off every attached display, or larger than every
  attached display, the window uses its default size and position.

Reconciling the position does not overwrite what is remembered; only moving or
resizing the window does, so reattaching the display restores the arrangement.

## Quit while agents are working

If you quit Knot while one or more agents are Working, Knot shows a warning
before terminating. The warning names one working agent, or states how many are
working, and stays until you choose an action.

- **Cancel** closes the warning, leaves Knot open, and lets every agent session
  keep running.
- **Quit** shuts Knot down through its normal path, without showing the warning
  again.

If no agent is Working, Knot quits without showing the warning. If the working
agents finish while the warning is open, it still applies: Quit terminates and
Cancel leaves Knot open.

## Source contracts

- openspec/specs/workspace-manager-ui/spec.md
- openspec/specs/dashboard/spec.md
- openspec/specs/window-lifecycle/spec.md
- openspec/specs/quit-warning/spec.md

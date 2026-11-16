# dashboard Specification

## Purpose

Defines the card-grid view of agents: the in-place dashboard a workspace
window toggles to in place of its terminal view, the separate Command Center
window that shows the same grid across workspaces, how each agent's card is
laid out and sorted, and the tile that creates a new agent from the grid.

## Requirements

### Requirement: Dashboard launcher

The workspace window SHALL show the dashboard launcher as a labelled row at
the top of its agent list - an icon and the word "Dashboard" - shaped like
the agent rows it sits above. Clicking it toggles the workspace window's own
content between its terminal view and the dashboard view; it does not open a
separate window. The row SHALL show a selected background while the dashboard
view is the one being shown, as an agent row does when selected.

It is deliberately not an icon-only button in the bottom bar beside "New
agent". The dashboard is the workspace's overview of every agent, so it
belongs above them and shaped like them; as a bare icon among the window's
controls it read as a minor one and went unnoticed. This follows the Swift
reference's `overviewRow` (`Skwad/Views/Sidebar/SidebarView.swift`).

#### Scenario: The launcher sits above the agent rows

- **WHEN** a workspace window is open
- **THEN** a labelled "Dashboard" row is visible at the top of the agent
  list, above the first agent row, and not in the bottom bar beside "New
  agent"

#### Scenario: The launcher marks which view is showing

- **WHEN** the workspace window is showing the dashboard view
- **THEN** the Dashboard row is drawn with a selected background
- **WHEN** it is showing the terminal view
- **THEN** the Dashboard row is drawn without one

#### Scenario: Launcher toggles the in-place view

- **WHEN** a workspace window is open and the Dashboard row is clicked
- **THEN** that window's content switches from the terminal view to the
  dashboard view, scoped to that workspace's agents
- **AND** clicking a card, or clicking the row again, switches back to
  the terminal view

### Requirement: Command Center window

A separate, global dashboard window ("Command Center") SHALL show every
attached workspace's agents, reusing the same agent card grid as the
workspace-scoped dashboard.

It SHALL be reachable from the Window menu and its key equivalent, per
`app-menu`, as well as from the workspace manager's toolbar. Only one
Command Center window SHALL exist at a time; a second request to open it
activates the one already open, per `window-lifecycle`.

Its content SHALL scroll vertically when the workspace sections and their
cards are taller than the window's content area, so every workspace and every
card remains reachable at any window size. Cards SHALL NOT be shrunk or
dropped to make the grid fit. The card grid is the Command Center's whole
purpose, and a user with several workspaces reaches the bottom of it in the
window's default size.

#### Scenario: Command Center shows all workspaces

- **WHEN** the Command Center window is open
- **THEN** every attached workspace appears as its own section in the
  grid, not just one workspace

#### Scenario: Card in Command Center opens the workspace window

- **WHEN** an agent's card is clicked in the Command Center
- **THEN** that agent's workspace window opens (or focuses, if already
  open) showing its terminal view with that agent selected

#### Scenario: The grid scrolls when it overflows

- **WHEN** the Command Center holds more workspace sections and cards than
  fit its content area
- **THEN** the content scrolls vertically, and the last card of the last
  workspace can be scrolled to and clicked

#### Scenario: A short grid does not scroll

- **WHEN** the Command Center's content is shorter than its content area
- **THEN** the content does not scroll and no scrollbar occupies space
  the cards would otherwise use

#### Scenario: Resizing reveals the rest

- **WHEN** the user shrinks the Command Center window until its content
  overflows
- **THEN** the content becomes scrollable rather than clipped

### Requirement: Agent card grid

The dashboard SHALL show agents grouped by workspace, each in a card
showing its avatar, name, status text and color, folder name, and git
diff stats, and, for an agent that is not a shell agent, a quick prompt
field, with an "Add Agent" tile per workspace group. A card SHALL show an
agent as not running when it has no live session: a terminal session, or a
panel session connecting or ready, in its open workspace window. A panel
session whose connection failed is not live.

#### Scenario: Card reflects agent state

- **WHEN** an agent's automatic state is `Running`
- **THEN** its card shows the orange status indicator and the current
  status/title text

#### Scenario: Card shows git diff stats

- **WHEN** an agent's folder has uncommitted changes
- **THEN** its card shows insertions/deletions/file counts parsed via
  `knot_git::parse_numstat`

#### Scenario: Empty workspace

- **WHEN** a workspace has no non-companion agents
- **THEN** its section shows "No agents" instead of a card grid

#### Scenario: An agent whose workspace window is closed

- **WHEN** the Command Center shows an activated agent whose workspace window
  is closed
- **THEN** its card shows it as not running

#### Scenario: A failed connection is not running

- **WHEN** an agent's panel session failed to connect
- **THEN** its card shows it as not running and its quick prompt is disabled,
  since a prompt queued there would never be sent

#### Scenario: A shell agent's card has no quick prompt

- **WHEN** a card shows a shell agent
- **THEN** it has no quick prompt field, and it is as tall as the cards
  beside it

### Requirement: Sort modes

The dashboard SHALL support sorting each workspace's agent cards by
manual (store order), name, or status.

#### Scenario: Sort by name

- **WHEN** the user selects "Name" in the sort picker
- **THEN** each workspace's cards are ordered alphabetically by agent name

### Requirement: Add Agent tile

Each workspace section SHALL show an "Add Agent" tile that opens the
existing agent-creation dialog, prefilled for that workspace.

#### Scenario: Add Agent opens prefilled dialog

- **WHEN** the user clicks "Add Agent" in a workspace's section
- **THEN** the agent editor opens with the folder prefilled from an
  existing agent in that workspace, if any

### Requirement: Dashboard status summary

The dashboard SHALL show a status summary counting agents by the state their
cards show: awaiting input, working, idle, error, then not running, leaving
out any state with no agents and not counting shell agents. Each workspace
section SHALL show one for its agents. The Command Center SHALL also show one
in its header for every agent it shows; a workspace's own dashboard, having
one section, SHALL NOT repeat it in its header.

#### Scenario: A workspace with mixed states

- **WHEN** a workspace section holds two working agents, one idle agent and
  one awaiting input
- **THEN** its summary reads one awaiting input, two working, one idle, in
  that order, and shows no error count

#### Scenario: A stopped agent counts as not running

- **WHEN** an agent has no live session, whatever state it last recorded
- **THEN** the summary counts it as not running, as its card shows it

#### Scenario: One summary in a workspace's own dashboard

- **WHEN** a workspace window shows its dashboard
- **THEN** its single section shows a summary and the header shows none

#### Scenario: Shell agents are not counted

- **WHEN** a workspace holds a shell agent and one idle agent
- **THEN** its summary counts one idle agent and nothing else

### Requirement: Quick prompt sends through the agent's own delivery path

A quick prompt SHALL be delivered exactly as the agent's composer delivers a
prompt typed by the user: sent when the agent is free, queued under the
prompt queue's rules when it is busy, and recorded in its conversation.
Enter SHALL send; Cmd+Enter SHALL send and then open the agent's workspace
window with that agent selected. Text that is empty or only whitespace SHALL
send nothing. The field SHALL clear once the prompt is delivered or queued.

#### Scenario: Sending to an idle agent

- **WHEN** the user types a prompt in an idle agent's quick prompt field and
  presses Enter
- **THEN** the prompt is sent to that agent, appears in its conversation as
  the user's message, and the field clears

#### Scenario: Sending to a working agent queues

- **WHEN** the user sends a quick prompt to an agent whose turn is in
  progress
- **THEN** the prompt joins that agent's prompt queue as the user's, and is
  delivered when the turn ends

#### Scenario: Send and open

- **WHEN** the user sends a quick prompt with Cmd+Enter
- **THEN** the prompt is delivered and the agent's workspace window opens or
  comes forward with that agent selected

#### Scenario: Sending from the Command Center

- **WHEN** the user sends a quick prompt from a card in the Command Center
- **THEN** it is delivered by the workspace window that holds the agent's
  session, the same as if it were sent there

### Requirement: Quick prompt says when it cannot send

A quick prompt field SHALL be disabled, saying the agent is not running, when
the agent has no live session. Whenever a prompt would queue rather than send
now (a turn in progress, a pending permission request, a usage-limit hold),
the field SHALL stay enabled and say it will queue. A delivery that finds no
session or no window SHALL keep the typed text.

#### Scenario: A stopped agent

- **WHEN** a card shows an agent with no session
- **THEN** its quick prompt field is disabled and says the agent is not
  running

#### Scenario: A working agent

- **WHEN** a card shows a working agent
- **THEN** its quick prompt field accepts input and says the prompt will
  queue

#### Scenario: Command Center, workspace window closed

- **WHEN** the Command Center shows an activated agent whose workspace window
  is closed
- **THEN** its quick prompt field is disabled and says the agent is not
  running, and nothing is sent

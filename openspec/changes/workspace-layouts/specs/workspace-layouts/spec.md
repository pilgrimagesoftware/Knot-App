## Purpose

Defines the panes a workspace window's content area shows: up to four agents
at once in one of five layouts, the dividers between them, which pane is
focused, and how selecting, creating and removing agents change the layout.

## ADDED Requirements

### Requirement: Layout modes

A workspace SHALL be in one of five layout modes: single (one pane), split
vertical (two panes side by side), split horizontal (two panes stacked),
three-pane, and four-pane grid. Each pane SHALL show exactly one agent of
that workspace, and no agent SHALL be in two panes. The layout and its
agents SHALL be per workspace.

#### Scenario: Each mode's pane count

- **WHEN** a workspace is in split vertical, split horizontal, three-pane or
  four-pane grid
- **THEN** its window shows 2, 2, 3 or 4 panes respectively, each holding a
  different agent of that workspace

#### Scenario: Layouts are per workspace

- **WHEN** workspace A is in a split and workspace B is in single
- **THEN** A's window shows two panes and B's window shows one

### Requirement: Pane geometry

The panes SHALL be placed by their order, using the primary ratio `r` and the
secondary ratio `s`. Split vertical: pane 0 on the left `r` of the width,
pane 1 on the right. Split horizontal: pane 0 on the top `r` of the height,
pane 1 below. Three-pane: pane 0 is the full-height left column of width `r`;
the right column is split at `s` of the height into pane 1 (top) and pane 2
(bottom).

#### Scenario: Three-pane arrangement

- **WHEN** a workspace is in three-pane with ratios 0.5 and 0.5 and agents
  A, B, C in that order
- **THEN** A fills the left half at full height, B the top right quarter and
  C the bottom right quarter

### Requirement: Four-pane grid geometry

In the four-pane grid, a vertical divider at `r` of the width and a
horizontal divider at `s` of the height SHALL each run the full height or
width, giving pane 0 at top left, pane 1 at top right, pane 2 at bottom left
and pane 3 at bottom right.

#### Scenario: One horizontal line across both columns

- **WHEN** the user drags the grid's horizontal divider
- **THEN** both the left and the right column change their split, together

### Requirement: Pane contents

A pane SHALL show what its agent shows when it is the only one: its live
terminal for a Terminal-mode agent, its conversation panel for a Panel-mode
agent, and the stopped view for an agent that is not running. Each pane in a
multi-pane layout SHALL show its agent's name above its content. This
diverges from the Swift app, whose panes held terminals only.

#### Scenario: Mixed agents side by side

- **WHEN** a split pairs a Panel-mode agent with a shell companion
- **THEN** one pane shows the conversation panel and the other the shell's
  live terminal, each under its agent's name

### Requirement: Showing an agent in a pane starts it

Placing an agent in a pane by a user action - a sidebar click, the number
shortcut, a Layout menu choice that fills panes, or creating a companion -
SHALL start it if it is not running, as selecting it in a single layout does.
Restoring a saved layout SHALL NOT: a restored pane starts its agent only as
its activation mode starts it on open, the way a restored selection does
today, and shows the stopped view otherwise.

#### Scenario: Filling a layout starts the agents it shows

- **WHEN** agents B and C are not running and the user switches agent A's
  window from Single to Three Panes
- **THEN** B and C are started, and all three panes show live agents

#### Scenario: A restored passive agent stays stopped

- **WHEN** a workspace was closed in a split showing A and passive agent B,
  and its window is reopened after a relaunch
- **THEN** the split is restored, and B's pane shows the stopped view until
  the user starts B

#### Scenario: An agent that stops while shown

- **WHEN** a split shows A and B, and B's session ends
- **THEN** B's pane shows the stopped view and the layout is unchanged

### Requirement: Dividers

Each boundary between panes SHALL be a draggable divider: one in a split, two
in three-pane (the horizontal one spanning the right column only) and two in
the grid. A divider SHALL show a resize cursor for its direction. Dragging
SHALL move its boundary live and SHALL clamp its ratio to between 0.25 and
0.75.

#### Scenario: A drag is clamped

- **WHEN** the user drags a split's divider to 10% of the width
- **THEN** the divider stops at 25% and the left pane is a quarter of the
  width

### Requirement: Ratios are saved when a drag ends

A divider's ratio SHALL be persisted once, when the drag ends, not on every
movement. The ratios SHALL be kept across mode changes, so returning to a
mode restores its dividers where they were left.

#### Scenario: A drag survives a restart

- **WHEN** the user drags a divider to 0.6, releases it, and restarts the app
- **THEN** the workspace's window opens with the divider at 0.6

### Requirement: The Layout menu

The content header SHALL offer a Layout menu while the workspace has at
least two agents and the agent view is showing. It SHALL list Single, Split
Vertical and Split Horizontal, plus Three Panes with three or more agents and
Four Panes with four or more. The current mode SHALL be checked. This
diverges from the Swift app, which offers Four Panes with three agents and
leaves a pane empty.

#### Scenario: Two agents

- **WHEN** a workspace has two agents and the user opens the Layout menu
- **THEN** it lists Single, Split Vertical and Split Horizontal, and not
  Three Panes or Four Panes

#### Scenario: One agent

- **WHEN** a workspace has one agent
- **THEN** no Layout menu is shown

### Requirement: Changing the layout mode

Choosing a mode SHALL keep the agents already in panes in their order. It
SHALL drop the agents past the new pane count, or fill the new panes with the
workspace's other agents in sidebar order. Choosing Single SHALL keep only the
focused pane's agent. Focus SHALL stay on its agent if still shown, else pane
0. Swift fills only some shortfalls and never moves focus.

#### Scenario: Single to split fills from the sidebar

- **WHEN** agent B of workspace agents A, B, C is shown alone and the user
  chooses Split Vertical
- **THEN** B is in pane 0 and A, the first other agent in sidebar order, is
  in pane 1

#### Scenario: Back to single keeps the focused agent

- **WHEN** a three-pane layout shows A, B, C with pane 2 focused and the user
  chooses Single
- **THEN** only C is shown

### Requirement: The focused pane

Exactly one pane SHALL be focused. Its agent SHALL be the window's selected
agent: the sidebar highlights it, the menus and shortcuts act on it, and the
git and artifact panels show it. Clicking in a pane SHALL focus it. Every
other pane SHALL be dimmed. A pane that receives keyboard input SHALL be the
focused one.

#### Scenario: Clicking a pane selects its agent

- **WHEN** a split shows A (focused) and B, and the user clicks in B's pane
- **THEN** B's pane is focused, A's is dimmed, B is highlighted in the
  sidebar, and the git panel, if open, shows B's folder

### Requirement: Selecting an agent from the sidebar

Clicking an agent row SHALL apply the first rule that matches:

1. R1: the agent has companions, pane 0 is focused, and ⌘ is not held: show
   its companion layout.
2. R2: the agent is in a pane: focus that pane.
3. R3: pane 0's agent has companions: show the clicked agent alone.
4. R4: put it in the focused pane, replacing that pane's agent.

⌘ skips only R1. The Swift app checks R3 before R2.

#### Scenario: Replacing the focused pane

- **WHEN** a split shows A and B with B focused, and the user clicks C, which
  owns no companions
- **THEN** the split shows A and C, with C focused

#### Scenario: Focusing an agent already shown

- **WHEN** a split shows A (focused) and B, and the user clicks B in the
  sidebar
- **THEN** the layout is unchanged and B's pane is focused

#### Scenario: Clicking a companion that is shown

- **WHEN** owner A and its companion S are shown with A focused, and the user
  clicks S's row
- **THEN** the layout is unchanged and S's pane is focused

#### Scenario: ⌘-click skips only the companion layout

- **WHEN** owner A and its companion S are shown with A in pane 0, and the
  user ⌘-clicks owner B, which has companions
- **THEN** R1 is skipped and R3 applies: B is shown alone

#### Scenario: Leaving a companion layout

- **WHEN** owner A and its companion S are shown and the user clicks C
- **THEN** C is shown alone

### Requirement: The sidebar highlights the focused agent's row

The sidebar SHALL highlight the row of the focused pane's agent, and no other
row. A focused companion SHALL highlight its own row, not its owner's. The
Swift app highlights the owner instead, since it lists no companion rows.

#### Scenario: A focused companion

- **WHEN** owner A and its companion S are shown and S's pane is focused
- **THEN** S's row is highlighted and A's is not

### Requirement: Selecting an agent by number in a layout

The agent-number shortcut (`keybindings`, "Selecting an agent by number")
SHALL act exactly as a click on that row without ⌘, applying the rules of
"Selecting an agent from the sidebar". Rows are counted as the sidebar shows
them, companion rows included. The Swift app counts only non-companion rows.

#### Scenario: The number of an agent already shown

- **WHEN** the sidebar lists A, B, C, a split shows A (focused) and B, and the
  user presses ⌘2
- **THEN** the layout is unchanged and B's pane is focused

### Requirement: Companion layout

Showing an agent with its companions SHALL put the agent in pane 0 and its
first three companions after it, choosing single, split vertical, three-pane
or grid for zero, one, two or three companions, and SHALL focus pane 0.
⌘-clicking the agent SHALL skip this and treat the click as an agent without
companions.

#### Scenario: An owner with two companions

- **WHEN** the user clicks owner A, which has companions S1 and S2
- **THEN** the window shows three-pane with A, S1, S2 and A focused

### Requirement: A new companion opens beside its creator

Creating a companion while its owner is in a pane SHALL show it beside the
owner. From single, the layout SHALL become split vertical with owner and
companion. From a split it SHALL
become three-pane and from three-pane a grid, adding the companion as the
last pane. In a full grid the companion SHALL replace the last pane that is
not its owner's. The companion's pane SHALL be focused.

#### Scenario: From single

- **WHEN** owner A is shown alone and the user creates a shell companion
- **THEN** a split vertical shows A on the left and the companion on the
  right, with the companion focused

#### Scenario: From a full grid

- **WHEN** a grid shows A, B, C, D in panes 0 to 3, and a companion of A is
  created
- **THEN** the companion replaces D in pane 3, the last pane that is not A's,
  and is focused

#### Scenario: The owner holds the last pane

- **WHEN** a grid shows B, C, D, A in panes 0 to 3, and a companion of A is
  created
- **THEN** the companion replaces D in pane 2 and is focused

### Requirement: A companion whose owner is not shown

When the owner is in no pane, a companion the user creates (the agent row's
New Shell Companion or New Companion…) SHALL replace the layout with a split
vertical of owner and companion, the companion focused. A companion created
any other way, such as by an agent over MCP, SHALL leave the layout
unchanged; it appears in the sidebar only.

#### Scenario: Created from the row of an owner that is not shown

- **WHEN** a split shows B and C, and the user chooses New Shell Companion on
  owner A's row
- **THEN** a split vertical shows A and the companion, with the companion
  focused

#### Scenario: Created over MCP while the user looks elsewhere

- **WHEN** a split shows B and C, and agent A creates a companion for itself
  over MCP
- **THEN** the split still shows B and C, and the companion is listed in the
  sidebar after A

### Requirement: Removal collapses the layout

Removing an agent that is in a pane SHALL remove its pane: a grid left with
three agents SHALL become three-pane, any multi-pane layout left with two a
split vertical, and any layout left with fewer than two single. The focused
pane SHALL be clamped to the panes that remain. A single layout whose agent
is removed SHALL show the workspace's first remaining agent. An owner's
companions are removed first (`agent-lifecycle`, "Agent removal").

#### Scenario: Grid loses a pane

- **WHEN** a grid shows A, B, C, D and C is removed
- **THEN** a three-pane layout shows A, B, D

#### Scenario: Split loses a pane

- **WHEN** a split shows A and its shell companion S, and S's shell exits
- **THEN** A is shown alone

### Requirement: Moving an agent to another workspace

Moving an agent to another workspace SHALL remove it from its old
workspace's layout as removal does, and SHALL NOT change the new workspace's
layout except to show the agent when the new workspace showed nothing. Swift
neither collapses the old layout nor touches the new one.

#### Scenario: Moved out of a split

- **WHEN** a split shows A and B and B is moved to another workspace
- **THEN** A is shown alone, and the other workspace's layout is unchanged

### Requirement: Terminals resize when a drag ends

During a divider drag, a terminal in a pane SHALL keep its row and column
count. When the drag ends, every terminal in a pane whose size changed SHALL
be resized once to fit, and its program SHALL see the new size. A
conversation panel SHALL reflow live. This is new: Swift relays terminals out
on every drag frame.

#### Scenario: One resize per drag

- **WHEN** the user drags the divider between two terminal panes across a
  hundred pixels and releases it
- **THEN** each terminal's program receives one resize, after the release

### Requirement: The saved layout is restored

A workspace's window SHALL open with its saved layout mode, agents, focused
pane and ratios, and every change to them SHALL be persisted. On load, agents
no longer in the workspace SHALL be dropped, the mode SHALL be reduced to fit
the agents that remain, the focused pane and ratios SHALL be clamped, and an
unknown mode SHALL read as single. This diverges from the Swift app, which
replaces the saved layout with a companion layout at launch.

#### Scenario: A saved split reopens

- **WHEN** a workspace is closed in a three-pane layout with pane 1 focused
  and reopened
- **THEN** it shows the same three agents in the same panes with pane 1
  focused

#### Scenario: A saved agent was removed

- **WHEN** a workspace's saved split names an agent that no longer exists
- **THEN** the window opens in single with the remaining agent

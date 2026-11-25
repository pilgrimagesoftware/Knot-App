# Spec Delta

## MODIFIED Requirements

### Requirement: Agent row context menu
Right-clicking an agent row in the workspace sidebar SHALL open a context
menu scoped to that agent, with its items in the order and groups given by
the scenario "The full menu", the groups separated by dividers. An item
listed there with a Shift alternate SHALL be one line showing either title,
per "Shift alternates in the agent row context menu", and the alternate
SHALL NOT appear as a line of its own. A divider SHALL NOT render when the
group it would separate is empty.

#### Scenario: The full menu
- **WHEN** a row's agent satisfies every visibility rule
- **THEN** the menu lists, in this order: New Companion…, New Shell
  Companion | Edit Agent…, Fork Agent, Duplicate Agent | Move to Workspace,
  Save to Bench, Bench Agent… | Open In…, Markdown Files, Config Files |
  Agent Info…, Register Agent, Lock Agent or Unlock Agent, Turn Off
  Autopilot or Turn On Autopilot, Deactivate (Shift: Deactivate with New
  Session), Restart Agent (Shift: Restart with New Conversation), Remove
  Agent - where `|` is a divider

#### Scenario: Right-click a non-companion agent
- **WHEN** the user right-clicks a non-companion, non-shell agent's row in
  a workspace that is the only attached workspace, and the agent has no
  markdown history
- **THEN** the menu shows New Companion…, New Shell Companion, Edit
  Agent…, Fork Agent, Duplicate Agent, Save to Bench, Bench Agent…, Open
  In…, Register Agent, Restart Agent and Remove Agent
- **AND** it does not show Move to Workspace (no other workspace to move
  to), Markdown Files (no history), or a separate Restart with New
  Conversation line

#### Scenario: Right-click a shell companion
- **WHEN** the user right-clicks a shell companion's row
- **THEN** the menu shows Edit Agent…, Open In… and Remove Agent
- **AND** it does not show New Companion…, New Shell Companion, Fork
  Agent, Duplicate Agent, Move to Workspace, Save to Bench, Bench Agent…,
  Register Agent or Restart Agent

#### Scenario: No divider around an empty group
- **WHEN** the user right-clicks a shell companion's row
- **THEN** the menu neither opens nor closes with a divider, and shows no
  two dividers in a row

### Requirement: Agent row context menu visibility rules
Each item's presence SHALL be decided by the agent it was opened on, as the
scenarios below state. Edit Agent…, Open In… and Remove Agent SHALL always
be shown. A Shift alternate that is not offered for an agent SHALL leave its
line showing the primary title whatever the modifiers, with no Shift
marker.

#### Scenario: A companion shows no owner-only items
- **WHEN** the user right-clicks a companion's row
- **THEN** New Companion…, New Shell Companion, Fork Agent, Duplicate
  Agent, Save to Bench, Bench Agent…, Move to Workspace and Restart Agent
  are absent, and no line offers a Shift alternate

#### Scenario: A shell agent hides Register Agent
- **WHEN** the user right-clicks a standalone shell agent's row
- **THEN** Register Agent, Lock Agent, Unlock Agent and the Autopilot items
  are absent from the menu

#### Scenario: Move to Workspace needs somewhere to move to
- **WHEN** the user right-clicks an agent in the only attached workspace
- **THEN** Move to Workspace is absent from the menu
- **WHEN** a second workspace is attached and the user right-clicks the
  same agent
- **THEN** Move to Workspace is present, and its submenu lists that second
  workspace and not the agent's own

#### Scenario: A shell agent hides Restart with New Conversation
- **WHEN** the user right-clicks a standalone shell agent's row and holds
  Shift
- **THEN** the line still reads Restart Agent, carries no Shift marker, and
  choosing it restarts the shell; Restart with New Conversation is offered
  neither as an alternate nor as a line

#### Scenario: A companion cannot be benched
- **WHEN** the user right-clicks a companion's row
- **THEN** Save to Bench and Bench Agent… are both absent

#### Scenario: Deactivate needs a running agent
- **WHEN** the user right-clicks the row of an agent that is not running
- **THEN** Deactivate is absent, and with it Deactivate with New Session

#### Scenario: Panel-only and history-only items
- **WHEN** the user right-clicks a terminal-mode agent with no markdown
  history and no config files
- **THEN** Agent Info…, Markdown Files and Config Files are absent

## ADDED Requirements

### Requirement: Shift alternates in the agent row context menu
A context menu line with a Shift alternate SHALL show its primary title
while Shift is up and its alternate's title while Shift is down, updating
the open menu as Shift changes. Choosing the line - by click, Return or
Shift-Return - SHALL run the action whose title is showing at that moment.
A line without an alternate SHALL NOT change its title or action with
Shift.

#### Scenario: Holding Shift swaps the title
- **WHEN** the context menu of a non-shell owner is open and the user holds
  Shift
- **THEN** the Restart Agent line reads Restart with New Conversation and
  the Deactivate line reads Deactivate with New Session
- **WHEN** the user releases Shift
- **THEN** the lines read Restart Agent and Deactivate again

#### Scenario: A Shift-click runs the alternate
- **WHEN** the user holds Shift and clicks the line reading Restart with New
  Conversation
- **THEN** the Restart with New Conversation confirmation appears, not
  Restart Agent's

#### Scenario: Shift-Return runs the alternate
- **WHEN** the Restart Agent line is highlighted and the user presses
  Shift-Return
- **THEN** the Restart with New Conversation confirmation appears

### Requirement: A Shift marker shows which lines have an alternate
A context menu line with a Shift alternate SHALL show a Shift marker (⇧) in
its trailing column whether or not Shift is down, so the alternate is
visible before the user tries the modifier. A line without one SHALL show
no marker.

#### Scenario: The marker shows which lines have an alternate
- **WHEN** the context menu of a running non-shell owner is open and no
  modifier is held
- **THEN** the Restart Agent and Deactivate lines show ⇧, and no other line
  does

# Spec Delta

## MODIFIED Requirements

### Requirement: Agent card grid

The dashboard SHALL show agents grouped by workspace, each in a card
showing its avatar, name, status text and color, folder name, and git
diff stats, and, for an agent that is not a shell agent, a quick prompt
field, with an "Add Agent" tile per workspace group. A card SHALL show an
agent as not running when it has no live session.

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

#### Scenario: A shell agent's card has no quick prompt

- **WHEN** a card shows a shell agent
- **THEN** it has no quick prompt field, and it is as tall as the cards
  beside it

## ADDED Requirements

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

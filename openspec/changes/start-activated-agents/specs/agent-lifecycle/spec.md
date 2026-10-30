# Spec Delta

## MODIFIED Requirements

### Requirement: Activation mode

Every agent SHALL carry an activation mode of `active` or `passive`, which
decides when its session starts on its own:

- An `active` agent SHALL start when its workspace opens.
- A `passive` agent SHALL NOT start when its workspace opens. It starts the
  first time the user selects it and its pane takes focus, and then stays
  running until it is deactivated, removed, or its workspace closes.

An agent shown as activated SHALL be running. An agent that becomes activated
while its workspace is open - created `active` there, or a companion of a
running owner - SHALL start then, not when it is next selected.

Activation mode SHALL NOT change what an agent is or how it runs once
started: an activated `passive` agent is indistinguishable from an `active`
one. Changing an agent's activation mode SHALL NOT restart it, and SHALL NOT
start or stop it.

#### Scenario: Opening a workspace starts only its active agents

- **WHEN** a workspace holding two `active` and three `passive` agents opens
- **THEN** the two `active` agents start
- **AND** the three `passive` agents do not, and no adapter subprocess is
  spawned for them

#### Scenario: Selecting a passive agent starts it

- **WHEN** the user selects a `passive` agent that has not been activated
- **THEN** that agent starts, and behaves from then on exactly as an `active`
  agent would

#### Scenario: A passive agent stays running once activated

- **WHEN** an activated `passive` agent is deselected and another agent is
  selected
- **THEN** the first agent goes on running

#### Scenario: Changing the mode of a running agent leaves it running

- **WHEN** a running agent's activation mode is changed from `passive` to
  `active`, or the other way round
- **THEN** the agent keeps running, its session intact, and its restart token
  is unchanged

#### Scenario: An agent created active into an open workspace starts

- **WHEN** another agent creates an `active` agent with `create-agent` while
  the workspace is open
- **THEN** the new agent starts without being selected, and a message sent to
  it is handled

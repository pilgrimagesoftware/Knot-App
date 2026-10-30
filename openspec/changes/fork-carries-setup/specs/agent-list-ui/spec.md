# Spec Delta

## MODIFIED Requirements

### Requirement: Fork Agent from the context menu
Selecting Fork Agent SHALL open the agent editor to create a new agent
carrying the selected agent's whole setup - folder, avatar, agent type,
persona, shell command, startup prompt, description, capabilities, cost tier,
activation mode and session setup (model, permission mode, effort) - its
session id (so the fork resumes the same conversation), a name suffixed to
mark it as a fork, and placement immediately after the source. Every carried
field the editor shows SHALL be editable before the agent is created. The
session id and session setup SHALL be carried only when the agent is created
with the source's agent type; a fork the user changed to another type SHALL
start a fresh session with that type's defaults.

#### Scenario: Fork carries the session forward
- **WHEN** the user selects Fork Agent on an agent with session id `s1`
- **THEN** the agent editor opens pre-populated from that agent, carrying
  `s1`, and the created agent is placed immediately after the source

#### Scenario: Cancelling a fork creates nothing
- **WHEN** the user selects Fork Agent and dismisses the editor
- **THEN** no agent is created

#### Scenario: Fork carries the setup
- **WHEN** the user selects Fork Agent on an agent with a description,
  capabilities, a cost tier, an activation mode and a chosen model, and
  creates the fork without changing anything
- **THEN** the created agent has the same description, capabilities, cost
  tier, activation mode and session setup as the source

#### Scenario: A fork changed to another type starts fresh
- **WHEN** the user forks a Claude agent and changes the type to Codex in the
  editor before creating
- **THEN** the created agent resumes no session and replays no session setup,
  and still carries the source's description, capabilities and cost tier

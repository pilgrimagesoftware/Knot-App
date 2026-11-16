# Spec Delta

## ADDED Requirements

### Requirement: A start attaches to the agent's running adapter

The system SHALL run at most one ACP adapter for an agent at a time. A start
requested while the agent's session is connecting or connected SHALL attach to
that session and SHALL NOT spawn another adapter, whichever path asks:
selecting the agent, a direct message, the activation sweep, a reconnect, a
restart or a plan step's fresh context.

#### Scenario: A message arrives while the agent is connecting

- **WHEN** an agent's session is connecting and a direct message to it asks
  for it to be started
- **THEN** no second adapter is spawned, and the message is read by the
  session already connecting

### Requirement: Teardown stops the adapter in any phase

Tearing an agent's session down SHALL stop its adapter whether the session is
connected or still connecting. A connection that is still being made SHALL be
abandoned, and the adapter it spawned SHALL be terminated. A torn-down
connection SHALL NOT become the agent's session, send a turn, or record its
session id for resuming.

#### Scenario: Restarting an agent that is still connecting

- **WHEN** an agent is restarted while its session is still connecting
- **THEN** the adapter that connection spawned is terminated, and only the
  restarted session's adapter keeps running

#### Scenario: A fresh-context step for an agent just woken by a message

- **WHEN** a plan step that asks for a fresh context is dispatched to a
  deactivated agent, the message starts it on its old conversation, and the
  fresh-context restart follows before that start has connected
- **THEN** the first start is stopped, and the agent runs one adapter, on the
  new conversation

#### Scenario: The teardown and the connection finish together

- **WHEN** a session's connection completes at the same moment the session
  is torn down
- **THEN** the connection does not become the agent's session, and its
  adapter is terminated

#### Scenario: Closing the window while an agent connects

- **WHEN** a workspace window closes while one of its agents is connecting
- **THEN** that agent's adapter is terminated and none keeps running after
  the window is gone

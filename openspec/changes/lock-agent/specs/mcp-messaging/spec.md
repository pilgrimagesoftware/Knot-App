# Spec Delta

## MODIFIED Requirements

### Requirement: Broadcast fan-out

Broadcast SHALL create one message per eligible recipient in the sender's
workspace (excluding the sender, unregistered agents, shell agents, locked
agents, and companion-rule violations). Each stored message SHALL follow the
same idle-gated delivery nudge as a direct send (see "Idle-time delivery
nudge"). The return value SHALL be the number of messages created. A sender
that may not send at all - unregistered, or locked - SHALL be rejected with the
same error a direct send from it gets, rather than reported as a broadcast that
reached no one.

#### Scenario: Broadcast to a mixed workspace

- **WHEN** a registered agent broadcasts in a workspace with two other
  registered non-shell agents and one shell agent
- **THEN** two messages are created and the count is 2

#### Scenario: Broadcast from an unregistered agent

- **WHEN** an agent that has not registered calls broadcast-message
- **THEN** the call is an error saying the sender is not registered, and
  nothing is stored

## ADDED Requirements

### Requirement: Locked agents cannot be messaged

An agent MAY be locked. A send to a locked agent SHALL be rejected with
"Recipient is locked and cannot receive messages", SHALL store nothing, and
SHALL NOT activate the recipient. Broadcast SHALL skip locked recipients.

#### Scenario: Message to a locked agent

- **WHEN** an agent sends a message to a locked, deactivated agent
- **THEN** the send is rejected, nothing is stored, and the recipient is not
  started

### Requirement: Locked agents cannot send messages

A send from a locked agent SHALL be rejected with "Locked agents cannot send
messages", and so SHALL a broadcast from one, creating no messages.

#### Scenario: Locked agent sends

- **WHEN** a locked agent calls send-message
- **THEN** the send is rejected and nothing is stored

#### Scenario: Locked agent broadcasts

- **WHEN** a locked agent calls broadcast-message
- **THEN** the call is an error saying locked agents cannot send messages, and
  nothing is stored

### Requirement: Locked agents are passed over for tasks and reported

`dispatch-task` SHALL NOT choose a locked agent for a capability-matched task,
and SHALL be rejected when the named assignee is locked. `list-agents` and
`describe-agents` SHALL report whether each agent is locked.

#### Scenario: Capability dispatch passes over a locked agent

- **WHEN** a task's capabilities match a locked agent and an unlocked one
- **THEN** the unlocked agent is assigned the task

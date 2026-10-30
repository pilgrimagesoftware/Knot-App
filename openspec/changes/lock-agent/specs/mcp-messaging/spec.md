# Spec Delta

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
messages", and a broadcast from one SHALL reach no one.

#### Scenario: Locked agent sends

- **WHEN** a locked agent calls send-message
- **THEN** the send is rejected and nothing is stored

### Requirement: Locked agents are passed over for tasks and reported

`dispatch-task` SHALL NOT choose a locked agent for a capability-matched task,
and SHALL be rejected when the named assignee is locked. `list-agents` and
`describe-agents` SHALL report whether each agent is locked.

#### Scenario: Capability dispatch passes over a locked agent

- **WHEN** a task's capabilities match a locked agent and an unlocked one
- **THEN** the unlocked agent is assigned the task

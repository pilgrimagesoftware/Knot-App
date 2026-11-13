# Spec Delta

## ADDED Requirements

### Requirement: A new conversation inherits the old one's open threads

When an agent is restarted into a new conversation, the system SHALL mark the
messages it held unanswered as unread again, and SHALL open the new
conversation's first turn with a note giving the agent's last status and the
senders and send times of those messages. A restart that keeps the
conversation SHALL do neither.

#### Scenario: A reset brief comes back

- **WHEN** an agent that read a brief and has not answered it is restarted
  with a new conversation
- **THEN** the brief is unread in its inbox, and its first turn names the
  brief's sender and its last status

#### Scenario: Keeping the conversation keeps the brief read

- **WHEN** the same agent is restarted keeping its conversation
- **THEN** the brief stays read and no note is sent

### Requirement: Knot's own reset waits for open threads

A restart into a fresh conversation that the system starts by itself, such as
a plan step's `resetContext`, SHALL wait while the agent holds an unanswered
message, and SHALL go ahead once it no longer does. A restart the user asks
for SHALL NOT wait.

#### Scenario: The reset waits for the reply

- **WHEN** a `resetContext` restart is due for an idle agent that holds an
  unanswered brief
- **THEN** it is not restarted until it has answered the brief's sender

#### Scenario: A completed task holds no later reset

- **WHEN** an agent finished its previous dispatched task with `complete-task`
  and no message, and a `resetContext` task is then dispatched to it
- **THEN** it is restarted on a new conversation as soon as it is idle

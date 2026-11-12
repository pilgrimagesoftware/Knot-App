# Spec Delta

## MODIFIED Requirements

### Requirement: Message model

A message SHALL carry a unique id, sender id, recipient id (both stored as
agent uuids), content, a timestamp, a read flag defaulting to false, the time
it was read (none while unread), whether it is one copy of a broadcast, and
whether it is a dispatched task's goal.
Messages SHALL be held in memory only and SHALL NOT persist across app
restarts.

#### Scenario: New message is unread

- **WHEN** a message is created
- **THEN** its read flag is false and it has a fresh id and timestamp

#### Scenario: Reading stamps the time

- **WHEN** a recipient checks its messages and they are marked read
- **THEN** each of them records when it was read

### Requirement: Check and mark-read

Check-messages SHALL return the caller's unread messages and, by default, mark
them read. A caller MAY request a non-destructive read that leaves the flags
unchanged, MAY ask for messages already read as well, and MAY limit the
result to messages sent at or after a given time. A check SHALL mark read
only the unread messages it returned. Unread queries SHALL report whether any
unread message exists and SHALL be able to return the most recent unread
message id.

#### Scenario: Check clears unread

- **WHEN** an agent checks messages with default options
- **THEN** it receives its unread messages and a subsequent unread query
  reports none

#### Scenario: A message left out is left unread

- **WHEN** an agent checks with a `since` later than an unread message was
  sent
- **THEN** that message is not returned and is still unread

## ADDED Requirements

### Requirement: A read message is owed a reply

A message SHALL be unanswered when its recipient has read it, it was sent
directly by another agent that still exists, it is neither a broadcast nor a
dispatched task's goal, and the recipient has sent no message to its sender
since reading it. Any message from the recipient to that sender SHALL settle
every message from that sender read before it.

#### Scenario: A reply settles the brief

- **WHEN** an agent reads a brief and then sends its sender a message
- **THEN** the brief is no longer unanswered

#### Scenario: Writing to someone else does not

- **WHEN** an agent reads a brief and then messages a third agent only
- **THEN** the brief is still unanswered

#### Scenario: A dispatched task's goal is never owed

- **WHEN** an agent reads the goal `dispatch-task` delivered and reports the
  task with `complete-task` alone
- **THEN** it holds no unanswered message

#### Scenario: A brief from an agent that is gone

- **WHEN** an agent reads a brief and its sender is then removed
- **THEN** the brief is not unanswered

#### Scenario: A broadcast is never owed

- **WHEN** an agent reads a broadcast and replies to no one
- **THEN** it holds no unanswered message

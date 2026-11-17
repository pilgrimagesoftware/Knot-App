# session-scheduling delta

## MODIFIED Requirements

### Requirement: A scheduled run never discards the agent's work

A run SHALL be skipped, and recorded as skipped with the reason, when the
agent is busy: it has a turn in progress, a pending permission request, a
pending question card, a queued prompt, or an unanswered message it holds.
The next occurrence is unaffected.

#### Scenario: Busy at run time

- **WHEN** a run comes due while the agent is working
- **THEN** the turn continues, no new conversation is started, and the run
  is recorded as skipped

#### Scenario: An unanswered brief

- **WHEN** a run comes due for an idle agent that holds an unanswered message
- **THEN** the run is skipped because the agent owes a reply, and the message
  stays as it was

#### Scenario: A question card waiting for an answer

- **WHEN** a run comes due while the agent's question card is pending
- **THEN** the run is skipped and recorded as skipped because a question is
  waiting for an answer, and the card stays as it was

### Requirement: Idle means nothing is happening and nobody is looking

An agent SHALL count as idle only while it has no turn in progress, no
running subagents, no queued prompt, no pending permission request, no
pending question card, no unanswered message it holds, and no running
background process its latest turn started. Typing, sending, or scrolling in
its conversation or terminal SHALL restart the idle period.

#### Scenario: Scrolling back counts as activity

- **WHEN** the user scrolls an agent's conversation 25 minutes into a
  30-minute idle period
- **THEN** the idle period starts again from that moment

#### Scenario: An unanswered brief keeps it awake

- **WHEN** an agent's turn has ended but it holds a message it has not
  answered
- **THEN** it is not idle, and is neither deactivated nor reset, until it
  answers

#### Scenario: A background process keeps it awake

- **WHEN** an agent's turn has ended but a test run it started is still
  running
- **THEN** the agent is not idle until that process ends

### Requirement: An agent that asked the user a question can go idle

An agent whose turn ended asking the user something in its reply text SHALL
count as idle, since nobody has answered. An agent waiting on a pending
permission request or a pending question card SHALL NOT: both hold the turn
open, and resetting or deactivating the agent would discard the action or
the answer it is waiting for.

#### Scenario: A question left unanswered

- **WHEN** an agent's turn ended with a question to the user and nobody
  answers for its idle period
- **THEN** it is stopped for idling

#### Scenario: A permission request left pending

- **WHEN** an agent has waited on a permission request for longer than its
  idle period
- **THEN** it is not stopped for idling

#### Scenario: A question card left pending

- **WHEN** an agent has shown a question card that nobody has answered for
  longer than its idle period
- **THEN** it is not stopped for idling, and the card stays

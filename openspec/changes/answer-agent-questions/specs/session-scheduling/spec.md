# session-scheduling delta

## ADDED Requirements

### Requirement: A pending question counts as busy
An agent with a pending question card (a form elicitation awaiting the user)
SHALL count as busy: a scheduled run that comes due SHALL be skipped and
recorded with the reason that a question is waiting for an answer, and idle
shutdown SHALL NOT deactivate or reset the agent while the question is
pending.

#### Scenario: A scheduled run waits for an answer
- **WHEN** a scheduled run comes due while the agent's question card is
  pending
- **THEN** the run is skipped and recorded as skipped because a question is
  waiting for an answer

#### Scenario: A pending question is not idle
- **WHEN** an agent with an idle policy has had a question card pending for
  longer than its idle time
- **THEN** the agent is neither deactivated nor reset

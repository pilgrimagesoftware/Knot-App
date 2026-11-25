## ADDED Requirements

### Requirement: Collapsing every tool call at once

The user SHALL be able to return every tool call in the selected agent's
conversation to its collapsed form in one action. Every summary run the user
opened SHALL return to its summary line, and every tool-call card drawn open
SHALL collapse to its header, including running and failed cards and cards
nested under a delegation. Only the user's open/closed choices SHALL change:
the conversation SHALL keep every call and result, and a call that arrives
afterwards SHALL follow its own default.

#### Scenario: Compact mode with opened runs

- **WHEN** compact mode is on, the user has opened two summary runs and a card
  inside one of them, and chooses Collapse Tool Calls
- **THEN** both runs are drawn as their summary lines again, and reopening a
  run shows its cards collapsed

#### Scenario: Compact mode off

- **WHEN** compact mode is off, one card is running, one failed and one was
  opened by the user, and the user chooses Collapse Tool Calls
- **THEN** all three cards show only their headers

#### Scenario: A call after collapsing

- **WHEN** the user collapses every tool call and the agent then starts a new
  one
- **THEN** the new call's card follows its default, open while it runs

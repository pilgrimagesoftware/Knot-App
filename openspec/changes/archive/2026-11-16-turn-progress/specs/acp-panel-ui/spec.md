# Spec Delta

## ADDED Requirements

### Requirement: The working turn shows its time and approximate size

While a turn is running, the working row SHALL show how long the turn has run,
and once the agent has streamed text in it, an approximate token count marked as
an estimate. The count SHALL cover only the running turn.

#### Scenario: A turn that has run 42 seconds

- **WHEN** a turn has run for 42 seconds and streamed about 4,800 characters
- **THEN** the working row reads "42s · ~1.2k tokens"

#### Scenario: A new turn

- **WHEN** the user sends another prompt
- **THEN** the time and count start again from zero

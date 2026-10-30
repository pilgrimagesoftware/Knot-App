# Spec Delta

## ADDED Requirements

### Requirement: A session limit is its own status

When a Panel-mode agent's turn ends in an error reporting that its session or
usage limit is reached, the system SHALL set the agent's status to Error until
the user sends another prompt, and every surface naming the status SHALL call
it "Session limit" rather than "Error", adding when the limit resets where the
provider said and there is room.

#### Scenario: A turn refused at the session limit

- **WHEN** a Panel-mode agent's prompt is answered with "You've hit your session
  limit · resets 5:20pm (America/Los_Angeles)"
- **THEN** the agent's dot is red in its cell and in the window's header, and
  the header reads "Session limit · resets 5:20pm (America/Los_Angeles)"

#### Scenario: The next prompt tries again

- **WHEN** the user sends a limited agent another prompt
- **THEN** the agent is Working, and no longer shows "Session limit"

#### Scenario: Any other failed turn

- **WHEN** a Panel-mode agent's prompt is answered with any other error
- **THEN** the agent goes Idle, as before

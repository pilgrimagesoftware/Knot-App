# Spec Delta

## ADDED Requirements

### Requirement: The agent editor sets a schedule and an idle policy

The agent editor SHALL let the user set a schedule (none, a preset, or a cron
expression) and an idle policy (use the default, off, or a period with
deactivate or reset). A schedule SHALL be refused while the agent has no
startup prompt, and an invalid cron expression SHALL be refused with a
message, with the next run shown for a valid one. Neither is offered for a
shell agent.

#### Scenario: Entering a cron expression

- **WHEN** the user enters `0 9 * * 1-5` as an agent's schedule
- **THEN** the editor shows the next run time, and saving keeps it

#### Scenario: No startup prompt

- **WHEN** the user sets a schedule on an agent with no startup prompt
- **THEN** the editor will not save it and says a startup prompt is needed

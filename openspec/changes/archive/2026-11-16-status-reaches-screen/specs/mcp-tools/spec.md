# Spec Delta

## MODIFIED Requirements

### Requirement: set-status

`set-status` SHALL require the caller's `agentId` and a `status` string. It
SHALL set the agent's human-readable status text; an empty string SHALL clear
it. This status is distinct from the automatic state machine value. The new
text SHALL be drawn wherever the agent's status is shown within one repaint
poll, whether or not anything else about that agent is changing.

#### Scenario: Clear status

- **WHEN** `set-status` is called with an empty `status`
- **THEN** the agent's status text becomes empty and its state is unchanged

#### Scenario: The status of an idle agent is drawn

- **WHEN** one agent calls `set-status` for another agent that is idle, with
  nothing streaming in any open window
- **THEN** the new text appears in the sidebar and everywhere else the status
  is shown, without the user doing anything

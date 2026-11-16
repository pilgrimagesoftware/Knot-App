# Spec Delta

## ADDED Requirements

### Requirement: Autopilot can be turned off for one agent

While Autopilot is enabled in Settings, the user SHALL be able to turn it off
for a single agent, and back on. The choice SHALL persist across a relaunch.
The system SHALL NOT classify a reply from an agent with Autopilot turned off,
whichever trigger fired, and SHALL drop a classification that was already in
flight when Autopilot was turned off for the agent. A duplicate or fork of the agent SHALL start following
Settings.

#### Scenario: A reply from an agent with Autopilot off

- **WHEN** Autopilot is enabled and configured, and an agent with Autopilot
  turned off finishes a turn
- **THEN** no classification runs and nothing lands for that agent

#### Scenario: Turned off while a reply is being classified

- **WHEN** the user turns Autopilot off for an agent while its last reply is
  being classified
- **THEN** the result is dropped: the agent is not marked, no notification is
  raised, and no reply is sent

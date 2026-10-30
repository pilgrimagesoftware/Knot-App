# Spec Delta

## ADDED Requirements

### Requirement: Autopilot indicator and per-agent toggle

An agent whose replies Autopilot reads - Autopilot on in Settings, not turned
off for the agent, and not a shell - SHALL show an Autopilot detail line with a
propeller icon on its sidebar row, and the same icon beside its status in the
title bar. The agent row context menu SHALL offer Turn Off Autopilot or Turn On
Autopilot, whichever applies, only while Autopilot is on in Settings and never
for a shell agent; the menu bar's Agents menu SHALL carry both.

#### Scenario: Turning Autopilot off for one agent

- **WHEN** Autopilot is on and the user selects Turn Off Autopilot on an agent
- **THEN** the agent's Autopilot line and title bar icon disappear, and its
  context menu offers Turn On Autopilot

#### Scenario: Autopilot off in Settings

- **WHEN** Autopilot is off in Settings
- **THEN** no agent shows the indicator, and the context menu offers neither
  item

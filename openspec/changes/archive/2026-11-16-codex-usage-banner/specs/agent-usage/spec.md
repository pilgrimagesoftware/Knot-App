# Spec Delta

## ADDED Requirements

### Requirement: A Codex agent near its usage limit says so

While a Codex agent is in a workspace window, the system SHALL read Codex's most
recent usage snapshot from Codex's own session files without blocking the UI. When
a usage window is at or above 75% used and has not reset since the snapshot, the
selected Codex agent SHALL show a banner saying the usage limit is approaching,
when that window resets, and a way to get more usage. Missing or unreadable usage
data SHALL show nothing and SHALL NOT be reported as an error.

#### Scenario: Codex near its limit

- **WHEN** Codex's latest snapshot shows its 5-hour window 82% used, resetting in
  an hour, and the user selects a Codex agent
- **THEN** a banner over the agent says the usage limit is approaching and when it
  resets, with Get more usage

#### Scenario: Another agent

- **WHEN** the user selects a non-Codex agent
- **THEN** no usage banner is shown

### Requirement: The usage banner can be dismissed until the reset

Dismissing the banner SHALL hide it until the window it warned about resets.

#### Scenario: Dismissed

- **WHEN** the user dismisses the banner
- **THEN** it is not shown again for that window's reset

# Spec Delta

## ADDED Requirements

### Requirement: Idle shutdown default

The settings window SHALL offer an idle shutdown default: off, or a period
with deactivate or reset. Off is the default. It applies to every agent whose
own idle setting is to use the default, and a change SHALL apply to open
windows without reopening them.

#### Scenario: Turning it on

- **WHEN** the user sets the default to deactivate after 45 minutes
- **THEN** every agent using the default is deactivated after 45 idle
  minutes, and agents with their own setting are unchanged

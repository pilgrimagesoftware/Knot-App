# Spec Delta

## ADDED Requirements

### Requirement: Adapters are checked for updates in the background

Knot SHALL check every ACP adapter it can install (the npm-installed ones)
for an update shortly after launch and then once a day while it runs,
without the user asking. A check that fails SHALL be retried at the next
interval and SHALL NOT stop later checks.

#### Scenario: A stale adapter is found without a click

- **WHEN** Knot has been running past its first check and a newer version of
  an installed adapter is published
- **THEN** the update is known to Knot by the next check, with no settings
  window open

### Requirement: A found update is announced once

When a check first finds an update to a given version of an adapter, Knot
SHALL raise one desktop notification for it, if desktop notifications are
enabled, and SHALL mark the Coding settings tab and that agent's tab with a
dot until the update is installed. A later check finding the same version
SHALL NOT notify again; a newer version SHALL. An update the user found by
checking from the Coding tab SHALL NOT also raise a notification.

#### Scenario: One notification per version

- **WHEN** two background checks in a row find version 1.4.0 of an adapter
  available
- **THEN** one notification is raised, not two

#### Scenario: Notifications off

- **WHEN** desktop notifications are disabled and a background check finds an
  update
- **THEN** no notification is raised, and the Coding tab still carries its dot

### Requirement: Adapter updates are logged

Knot SHALL write to the application log each update a check finds, each
adapter update it installs with the version installed, and each check or
install that fails, with the reason.

#### Scenario: An install is logged

- **WHEN** the user updates an adapter from the Coding tab
- **THEN** the application log records the adapter and the version now
  installed

# Proposal

## Why

On the Coding tab, an agent's options and its ACP adapter were two separate
groups under a generic "Coding agent" dropdown, so it was not obvious they
belonged to one agent (#367). And an adapter's update check only ever ran when
the user clicked its refresh button: an adapter going stale went unnoticed,
which is how #598's model-list mismatch arose, and nothing recorded updates in
the application log.

## What Changes

- The Coding tab picks an agent with a row of tabs, one per agent type, and
  shows that agent's options and its adapter together in one group named for
  the agent. The Source Folder section is unchanged.
- Knot checks every adapter it can update (the npm-installed ones) shortly
  after launch and then once a day, in the background.
- A newly found update raises one desktop notification per adapter and
  version, when desktop notifications are on, and puts a dot on the Coding
  tab and on that agent's tab. The Coding tab shows what the last check found,
  whether it ran in the background or on request.
- Found updates, installs and failed checks are written to the application
  log.

## Impact

- `knot`: a new `adapter_updates` module (the background check and its
  state), the Coding pane's layout, the settings tab badge.
- New l10n keys for the notification and the Coding tab.

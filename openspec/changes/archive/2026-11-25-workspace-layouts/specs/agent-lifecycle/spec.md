## MODIFIED Requirements

### Requirement: Shell companions

The system SHALL support shell companion agents bound to an owner agent via
created-by set to the owner id and is-companion true. A non-companion agent MAY
own companions; a companion MUST NOT own companions. Creating a shell companion
places it after its owner and shows it beside its owner in the workspace's
layout, as `workspace-layouts`' "A new companion opens beside its creator"
describes: from a single pane, a split layout pairing owner and companion.

#### Scenario: Companion is bound to its owner

- **WHEN** a shell companion is created for owner `A`
- **THEN** the companion's created-by is `A`, is-companion is true, agent type
  is `shell`, and it is inserted after `A`

#### Scenario: Companion opens beside its owner

- **WHEN** a shell companion is created for owner `A`, which is shown alone
- **THEN** a split layout shows `A` and the companion side by side

### Requirement: Agent removal

When an agent is removed, the system SHALL first remove every companion it owns,
then, if the agent is registered with MCP, unregister it, then tear down its
terminal session and any per-agent notification tracking, then remove it from
all workspaces and from the master agent list. Removal SHALL re-select or
collapse split panes so no pane references the removed agent, as
`workspace-layouts`' "Removal collapses the layout" describes.

#### Scenario: Removing an owner cascades to companions

- **WHEN** owner `A` with companions `B` and `C` is removed
- **THEN** `B` and `C` are removed first, then `A`
- **AND** none remain in any workspace or the master list

#### Scenario: Removing a registered agent unregisters it

- **WHEN** a registered agent is removed
- **THEN** an MCP unregister is issued for its id before teardown

#### Scenario: Removing an agent shown in a pane

- **WHEN** a split layout shows `A` and `B`, and `B` is removed
- **THEN** `A` is shown alone and no pane refers to `B`

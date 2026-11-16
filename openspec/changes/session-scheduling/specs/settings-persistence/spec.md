# Spec Delta

## MODIFIED Requirements

### Requirement: Preferences and durable data are stored separately

The system SHALL store the settings surface as three kinds of document, in two
directories, distinguished by what the values are rather than by which screen
edits them:

- **Preferences** - the scalar values: everything the user tunes about how the
  app looks and behaves, plus the per-agent-type command and options maps, the
  font settings, the sidebar width, the source base folder and its
  first-launch detection flag, the idle shutdown default, and the document
  version marker. These SHALL be
  stored as a single preferences document in the platform's user-preferences
  directory (on macOS, `~/Library/Preferences` under the application's
  directory).
- **Durable data** - the collections of objects the user created or that
  Knot recorded on their behalf: saved agents, saved workspaces, personas,
  bench templates, library prompts, saved library locations, recent
  repositories and recorded pull requests. These SHALL be stored in the platform's application-data
  directory (on macOS, `~/Library/Application Support` under the
  application's directory), as one document per collection: saved agents,
  workspaces, personas, bench templates, library prompts, saved library
  locations, recent repositories and recorded pull requests each in their own
  document.

  Recorded pull requests are durable data rather than a preference even though
  the user did not type them: they are a collection of objects with identity
  that grows without bound, which is what separates the two kinds here.
- **UI state** - what the application itself recorded about how its windows
  were last arranged, which the user never entered and would not miss if it
  were discarded. This SHALL be stored in the application-data directory, in
  its own document, separate from every durable-data collection.

A saved workspace SHALL hold only what the user configured about it: its
identity, its name, its color, and which agents belong to it. How that
workspace's window was last arranged is UI state and SHALL NOT be part of the
saved workspace record.

The two directories SHALL be derived from the same organization and
application identity the store already uses, so the preferences document and
the data documents name the same application.

On a platform where the two directories are the same, the split into separate
documents SHALL still hold; only their location coincides.

#### Scenario: Preferences live beside the platform's other preferences

- **WHEN** a scalar is written and the store's locations are inspected
- **THEN** the preferences document is under the platform's user-preferences
  directory, not the application-data directory

#### Scenario: Each collection is its own document

- **WHEN** a store holding agents, workspaces, personas, bench templates,
  library prompts, saved library locations, recent repositories and recorded
  pull requests is persisted
- **THEN** the application-data directory holds one document per collection,
  eight in all

#### Scenario: A fresh install writes only what it has

- **WHEN** a store with no persisted documents has a single scalar set
- **THEN** the preferences document is written and no collection document is
  required to exist for the store to load again

#### Scenario: A store with no recorded pull requests

- **WHEN** a store that predates the recorded-pull-request document is loaded
- **THEN** it loads successfully with no recorded pull requests, and no
  document for them is written until one is recorded

#### Scenario: A store with no prompt library

- **WHEN** a store that predates the library-prompt document is loaded
- **THEN** it loads successfully with an empty library, and no document for
  it is written until a prompt is added

#### Scenario: A store with no saved library locations

- **WHEN** a store that predates the library-location document is loaded
- **THEN** it loads successfully with no saved locations, the built-in
  Knot-Library is still available, and no document for saved locations is
  written until one is saved

#### Scenario: A saved workspace holds no UI state

- **WHEN** the workspaces document is written for a workspace whose window has
  been moved, resized, split and detached
- **THEN** the workspace's record holds its identity, name, color and agent
  membership, and none of that window arrangement

### Requirement: Per-workspace UI state is its own document

The system SHALL store, per workspace and keyed by the workspace's identity,
the UI state the application tracks on the user's behalf: the window's saved
bounds, the layout mode, which agents are active in that layout, the focused
pane, the layout's split ratios, whether the dashboard is showing, whether
the workspace is detached, and the last scheduled run started for each of its
agents.

Writing any of that SHALL persist the UI-state document and SHALL NOT rewrite
the saved-workspaces document, the saved-agents document, or any other. Where a
window sits is the most frequently written value in the store and the least
valuable; it SHALL NOT be a reason to rewrite what the user configured.

Reading the settings surface SHALL NOT require knowing which document a value
came from, as it does not for any other value.

An entry SHALL be decode-tolerant field by field: a document written before a
field existed SHALL load with that field defaulted rather than failing, and an
entry that cannot be decoded at all SHALL cost only that workspace's
arrangement. The layout mode SHALL default to a single pane and the split
ratio to the midpoint, which is what every record written before this document
existed already holds.

A missing or undecodable UI-state document SHALL leave every workspace loadable
with default arrangement. Losing this document SHALL never cost a workspace, an
agent, or a workspace's agent membership.

An entry keyed to a workspace that no longer exists SHALL be discarded rather
than retained indefinitely, so deleting a workspace does not leave its window
arrangement behind forever.

#### Scenario: Moving a window leaves user data alone

- **WHEN** the user drags a workspace's window to a new position
- **THEN** the UI-state document is written, and the saved-workspaces and
  saved-agents documents are byte-for-byte unchanged

#### Scenario: Arrangement survives a restart

- **WHEN** a workspace's window is moved, resized and split, and the app is
  restarted
- **THEN** the window reopens with the same bounds, layout mode and split
  ratios it had

#### Scenario: A missing UI-state document costs only arrangement

- **WHEN** the UI-state document is deleted and the app is started
- **THEN** every workspace loads with its name, color and agents intact, and
  its window opens with default arrangement

#### Scenario: An undecodable entry is contained

- **WHEN** the UI-state document holds one entry that cannot be decoded and
  others that can
- **THEN** the workspace with the bad entry opens with default arrangement and
  the others open with theirs

#### Scenario: A deleted workspace's arrangement is not kept

- **WHEN** a workspace is deleted and the store is reloaded
- **THEN** the UI-state document holds no entry for it

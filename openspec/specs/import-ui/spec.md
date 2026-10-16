# import-ui Specification

## Purpose

Defines the Import window - where an import is run from, what it shows before
it writes anything, and what it reports afterwards.

## Requirements

### Requirement: Import is its own window, not a setting

Knot SHALL offer its imports in a window of their own, opened from the File
menu, and SHALL NOT place them among the settings panes. An import is an
action the user runs at a moment they choose, not a preference they keep;
settings holds what Knot remembers, and an import changes the store once and
is done.

The window SHALL be a single instance: choosing Import again while it is open
SHALL raise the open window rather than opening a second one, so two windows
cannot offer the same records and import them twice.

The window SHALL read the store as it opens rather than a copy captured
earlier, so an import cannot overwrite a change saved since Knot started.

#### Scenario: Import is reachable from the File menu

- **WHEN** the user opens the File menu
- **THEN** it offers Import…, and choosing it opens the Import window

#### Scenario: Settings holds no import

- **WHEN** the user opens the settings window
- **THEN** no pane offers an import, and no tab is named Import

#### Scenario: Choosing Import twice

- **WHEN** the Import window is open and the user chooses Import… again
- **THEN** the open window is raised, and no second window appears

### Requirement: The window shows every source before it writes

The Import window SHALL hold one section per source Knot can import from:
personas from a coding-agent tool's subagent definitions, workspaces from
Skwad, and personas and prompts from a library (see `data-import` - Library
locations).

Each section SHALL show what its source holds, let the user select which
records to import, and report the result afterwards - what was added, what
was skipped as already present, and what could not be read - per
`data-import`.

A source with nothing to offer SHALL say so in place of its list, rather than
showing an empty list or being hidden. A hidden section is indistinguishable
from a section that does not exist, and the user cannot tell whether Knot
looked.

The window SHALL scan its sources when it opens and SHALL offer to scan them
again, so a definition written while the window is open can be imported
without reopening it. Scanning SHALL NOT happen while drawing a frame. For a
library, scanning means reading its index (see "The Library section reads
the chosen location before it lists").

#### Scenario: Importing personas from Claude

- **WHEN** the user opens the Import window with Claude subagent definitions
  present
- **THEN** the personas section lists them for selection, and importing the
  selected ones adds them and reports how many were added and skipped

#### Scenario: A source with nothing in it

- **WHEN** the user opens the Import window with no Skwad installation
  present
- **THEN** the Skwad section says there is nothing to import, rather than
  showing an empty list

#### Scenario: Unreadable records are named

- **WHEN** an import finishes with records it could not read
- **THEN** the section names them, rather than reporting a count alone

#### Scenario: A definition written while the window is open

- **WHEN** the user writes a new subagent definition and asks the window to
  look again
- **THEN** the new definition is offered, and any earlier selection is
  cleared rather than pointing at records that may no longer be there

### Requirement: An import always says what it did

Every import SHALL report its outcome on screen, and the report SHALL remain
visible without the user having to scroll or resize to find it. There is no
input for which an import reports nothing: one that added nothing says so,
and one that failed says so with the reason.

A failure SHALL be distinguishable from a success at a glance. An import that
failed and an import that did nothing otherwise read identically, and only
one of them needs acting on.

Reporting SHALL NOT depend on a delivery channel the user may not be able to
see. A message written only to standard output, or a desktop notification
that the platform suppresses outside an app bundle, does not satisfy this.

#### Scenario: An import that fails

- **WHEN** an import cannot complete
- **THEN** the window names the failure and its reason, drawn so it does not
  read as an ordinary summary

#### Scenario: An import with nothing to do

- **WHEN** every selected record is already present
- **THEN** the window says there was nothing to import, rather than showing
  an empty panel

#### Scenario: A summary that will not fit

- **WHEN** the sources hold more records than the window can show at once
- **THEN** the lists scroll and the summary stays in view, rather than the
  summary being pushed out of the window

### Requirement: An import reaches the running application

Imported workspaces and agents SHALL be added to the live store the open
windows render from, not only to the settings document. A record written to
settings alone is invisible until the application is restarted.

Every open window SHALL be scheduled to redraw when an import completes.
Rendering from shared state is not enough on its own: a window that is not
scheduled for a paint goes on showing the frame it last drew, so an import
that correctly updated the store still appeared to have done nothing.

The live copy of a record SHALL win over an incoming one: an agent the
application already holds has session state that an imported copy does not,
so a record whose identifier is already present is left as it is.

Adding to the store SHALL be idempotent, so importing again adds nothing a
second time.

#### Scenario: An imported workspace appears without a restart

- **WHEN** the user imports a workspace while the workspace manager is open
- **THEN** the workspace manager shows it, without the application being
  restarted

#### Scenario: An imported workspace survives the next save

- **WHEN** a workspace is imported and the user then renames, reorders or
  deletes any workspace
- **THEN** the imported workspace is still present afterwards

#### Scenario: An agent already running is not replaced

- **WHEN** an import offers a record whose identifier the application already
  holds
- **THEN** the held record is left exactly as it is, with its session intact

### Requirement: The Library section reads the chosen location before it lists

The Import window's Library section SHALL have a location picker. The
built-in Knot-Library comes first, then the saved locations by name. The
picker SHALL open on Knot-Library. Only the chosen location is read, so
saved locations that aren't chosen make no request.

The section SHALL list the chosen library's personas and prompts in two
groups, Personas and Prompts. Each row SHALL show the item's title and its
one-line description, in the order the index lists them.

The section SHALL read the chosen location's index when the window opens,
when another location is chosen, and when the user asks the window to look
again, never while drawing a frame. While it is reading, the section SHALL
say it is loading.

A library item Knot already holds (see `data-import` - Imported library
items keep their library identity) SHALL be listed, but SHALL NOT be
selectable, and its row SHALL say why: that it is already in Knot, or, for
a deleted built-in persona, that it can be brought back with the Personas
window's Restore Defaults.

When the index can't be read, or its format is one this Knot doesn't
support, the section SHALL say which, in place of its list, and SHALL offer
to try again. The other sections, and the other locations in the picker,
SHALL be unaffected.

#### Scenario: Listing the built-in library

- **WHEN** the user opens the Import window and Knot-Library is reachable
- **THEN** the Library section shows Knot-Library in the picker and a
  loading state, then lists its personas and prompts by title and
  description

#### Scenario: Switching location

- **WHEN** the user picks a saved location in the Library section
- **THEN** the section reads that location's index and lists its items in
  place of the previous location's, and any earlier selection is cleared

#### Scenario: Built-ins already in Knot

- **WHEN** Knot-Library lists the seven built-in personas and none has been
  deleted in Knot
- **THEN** all seven rows say they are already in Knot, and none of them can
  be selected

#### Scenario: Offline

- **WHEN** the user opens the Import window with no network
- **THEN** the Library section says it couldn't reach the library and
  offers to try again, while the Claude and Skwad sections list their
  sources as usual

#### Scenario: Importing a prompt from a library

- **WHEN** the user selects one library prompt and imports it
- **THEN** the prompt appears in the Prompts window, and the section reports
  one added

### Requirement: Library locations are managed from the Library section

The Library section SHALL offer to add a location. Adding asks for a name and
one of: a GitHub repository (`owner/repo`, with an optional branch), an
`https://` web address, or a folder on disk chosen with the platform's folder
picker. The input SHALL be checked for form before it is saved: a
well-formed `owner/repo`, an `https://` URL, an existing folder. The
location SHALL NOT need to be reachable to be saved. A newly saved location
SHALL become the chosen one.

The section SHALL let the user rename and remove saved locations. Removing
one SHALL ask for confirmation naming it, and SHALL NOT affect anything
already imported from it. Knot-Library SHALL offer neither action.

Before saving, adding a location SHALL say that its personas' and prompts'
text becomes agent instructions, so the user should only add locations they
trust.

#### Scenario: Adding a web address

- **WHEN** the user adds a location named "Mirror" for
  `https://example.com/knot-library`
- **THEN** "Mirror" appears in the picker, is chosen, and its index is read

#### Scenario: A malformed repository

- **WHEN** the user enters `acme` as a GitHub repository
- **THEN** the location isn't saved, and the dialog says a repository is
  written `owner/repo`

#### Scenario: Removing a location keeps its imports

- **WHEN** the user imports a persona from "Team" and then removes "Team"
- **THEN** the persona is still in Knot, and "Team" is gone from the picker

#### Scenario: The built-in location can't be removed

- **WHEN** Knot-Library is chosen in the picker
- **THEN** no rename or remove action is offered for it

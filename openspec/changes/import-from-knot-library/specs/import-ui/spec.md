# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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

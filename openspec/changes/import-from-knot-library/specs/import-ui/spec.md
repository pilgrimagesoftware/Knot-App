# Spec Delta

## MODIFIED Requirements

### Requirement: The window shows every source before it writes

The Import window SHALL hold one section per source Knot can import from:
personas from a coding-agent tool's subagent definitions, workspaces from
Skwad, and personas and prompts from Knot-Library.

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
without reopening it. Scanning SHALL NOT happen while drawing a frame. For
Knot-Library, scanning means fetching its index (see "The Knot-Library
section fetches before it lists").

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

### Requirement: The Knot-Library section fetches before it lists

The Import window's Knot-Library section SHALL list the library's personas
and prompts in two groups, Personas and Prompts. Each row SHALL show the
item's title and its one-line description. Rows SHALL be in the order the
library's index lists them.

The section SHALL fetch the index when the window opens and when the user
asks the window to look again, never while drawing a frame. While a fetch is
in flight, the section SHALL say it is loading.

A library item Knot already holds (see `data-import` - Imported library
items keep their library identity) SHALL be listed, but SHALL NOT be
selectable, and its row SHALL say why: that it is already in Knot, or, for
a deleted built-in persona, that it can be brought back with the Personas
window's Restore Defaults.

When the index can't be fetched, or its format is one this Knot doesn't
support, the section SHALL say which, in place of its list, and SHALL offer
to try again. The other sections SHALL be unaffected.

#### Scenario: Listing the library

- **WHEN** the user opens the Import window and the library is reachable
- **THEN** the Knot-Library section shows a loading state, then lists the
  library's personas and prompts by title and description

#### Scenario: Built-ins already in Knot

- **WHEN** the library lists the seven built-in personas and none has been
  deleted in Knot
- **THEN** all seven rows say they are already in Knot, and none of them can
  be selected

#### Scenario: Offline

- **WHEN** the user opens the Import window with no network
- **THEN** the Knot-Library section says it couldn't reach the library and
  offers to try again, while the Claude and Skwad sections list their
  sources as usual

#### Scenario: Importing a prompt from the library

- **WHEN** the user selects one library prompt and imports it
- **THEN** the prompt appears in the Prompts window, and the section reports
  one added

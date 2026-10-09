# Spec Delta

## ADDED Requirements

### Requirement: Library locations

Knot SHALL import personas and prompts from a library: any location that
publishes Knot-Library's format, meaning an `index.json` at its root and the
item files it lists. A location SHALL be one of:

- **A GitHub repository**, given as `owner/repo` with an optional branch.
  Its index is read from
  `https://raw.githubusercontent.com/<owner>/<repo>/<branch>/index.json`, and
  `HEAD` (the repository's default branch) is used when no branch is given.
- **A web address**: an `https://` URL of the directory that holds
  `index.json`. Plain `http://` SHALL be refused.
- **A folder on disk** that holds `index.json`, such as a local clone of a
  library being written.

The built-in location, Knot-Library (`pilgrimagesoftware/Knot-Library` on
branch `master`), SHALL always be available. It can't be edited or removed.
The user SHALL be able to save other locations, each with a name, and to
rename and remove them. Saved locations are durable data (see
`settings-persistence`).

An item's `path` from an index SHALL be resolved inside its location. A path
that is absolute, or that contains a `..` component, SHALL make that item
unreadable, so an index can't make Knot read outside the location.

#### Scenario: The built-in library needs no setup

- **WHEN** the user has never saved a location
- **THEN** Knot-Library is available to import from

#### Scenario: Saving a GitHub repository

- **WHEN** the user saves a location named "Team" for `acme/knot-personas`
  with no branch
- **THEN** Knot reads that library's index from the repository's default
  branch, and "Team" is still available after a restart

#### Scenario: A folder being written

- **WHEN** the user saves a folder that holds a generated `index.json` and
  its item files
- **THEN** Knot offers that folder's personas and prompts

#### Scenario: Plain HTTP is refused

- **WHEN** the user tries to save `http://example.com/library`
- **THEN** the location isn't saved, and Knot says only `https://` addresses
  are accepted

#### Scenario: A path that escapes the location

- **WHEN** an index lists an item at `../../.ssh/config`
- **THEN** that item is reported as unreadable, and nothing outside the
  location is read

#### Scenario: The built-in location stays

- **WHEN** the user removes every saved location
- **THEN** Knot-Library is still available

### Requirement: Library format

From a library's index, Knot SHALL offer every entry whose `kind` is
`persona` or `prompt`, and SHALL skip, without error, entries of any other
kind.

An item SHALL map onto Knot's records as Knot-Library's `content-format`
spec defines:

- A persona's `title` becomes `Persona.name` and its body becomes
  `Persona.instructions`.
- A prompt's `title` becomes `Prompt.name` and its body becomes
  `Prompt.text`.
- In both cases the item's `id` becomes the record's id.

The body is everything after the front matter's closing delimiter, trimmed.

Knot SHALL support index `format` 1. An index with any other `format` SHALL
be treated as unreadable as a whole, and nothing from it SHALL be offered.

A library is read-only to Knot. Importing SHALL NOT write to it.

#### Scenario: Personas and prompts are offered

- **WHEN** the index lists seven personas and three prompts
- **THEN** Knot offers all ten, each under its kind

#### Scenario: An unknown kind is skipped

- **WHEN** the index also lists items of kind `template`
- **THEN** Knot offers only the personas and prompts, and reports no error
  for the templates

#### Scenario: A newer index format

- **WHEN** the index's `format` is `2`
- **THEN** Knot offers nothing from that library and says it needs a newer
  Knot

#### Scenario: A prompt keeps its variables

- **WHEN** Knot imports a prompt whose body contains `{{folder.name}}`
- **THEN** the imported prompt's text contains `{{folder.name}}` exactly as
  written

### Requirement: Imported library items keep their library identity

A persona or prompt imported from a library SHALL take the item's `id` as
its own. For library sources, Knot already holds an item when it holds a
persona (for a persona item) or a prompt (for a prompt item) with that id,
in any state, including a soft-deleted built-in persona. This applies
whichever location the item comes from. A held item SHALL be skipped, per
"Import is additive and idempotent", whatever its name or contents.

An imported persona SHALL be a user persona (`PersonaType::User`), the same
as one imported from a subagent definition: editable, and removed outright
when deleted. Once it is deleted, its id is no longer held, and the item
can be imported again.

Restore Defaults SHALL NOT read any library. It restores the built-in
personas shipped with Knot.

#### Scenario: A built-in persona is already held

- **WHEN** the library lists Kent Beck under the same id Knot ships it with
- **THEN** the item is skipped as already present, and no second Kent Beck
  appears

#### Scenario: A renamed copy is still the same item

- **WHEN** the user imports a library prompt, renames it in Knot, and opens
  the import again
- **THEN** the prompt is still skipped as already present

#### Scenario: The same item from two locations

- **WHEN** the user imports a persona from one location, and a second
  location lists an item with the same id
- **THEN** the second location's item is skipped as already present

#### Scenario: A deleted built-in is not revived

- **WHEN** the user has deleted the built-in Linus Torvalds persona and the
  library lists it
- **THEN** the item is not imported, and the Linus Torvalds persona stays
  deleted

#### Scenario: Re-importing after deleting

- **WHEN** the user imports a library persona, deletes it, and imports it
  again
- **THEN** it is added again with the same id

### Requirement: Library downloads are verified

Knot SHALL read item files only when the user confirms an import, and only
for the items selected.

For a GitHub repository location, including the built-in one, Knot SHALL
fetch each item from
`https://raw.githubusercontent.com/<owner>/<repo>/<commit>/<path>`, using the
index's `commit`, so the item is the version the index describes. For a web
address or a folder, Knot SHALL read the item at `<location>/<path>`.

Before importing an item from any location, Knot SHALL check that its bytes
have the entry's `size` and that their SHA-256 matches its `sha256`. An item
that can't be read, fails the check, or has an empty body SHALL be reported
as unreadable by its title, and SHALL NOT be imported. The other selected
items SHALL still be imported (see "Partial failure does not abandon the
import").

#### Scenario: A pinned download

- **WHEN** new content merges into a GitHub library after Knot read the
  index, and the user then imports an item
- **THEN** Knot imports the version the index describes

#### Scenario: Bytes that don't match

- **WHEN** one selected item's bytes have a different SHA-256 than its index
  entry
- **THEN** that item is reported as unreadable by title, and the other
  selected items are imported

#### Scenario: A folder whose index is stale

- **WHEN** an item in a library folder was edited after its `index.json`
  was generated
- **THEN** that item is reported as unreadable, because its bytes no longer
  match the index

# Spec Delta

## ADDED Requirements

### Requirement: Knot-Library source and format

Knot SHALL import personas and prompts from Knot-Library
(`pilgrimagesoftware/Knot-Library`). It SHALL read the library's index from
`https://raw.githubusercontent.com/pilgrimagesoftware/Knot-Library/master/index.json`
and SHALL offer every index entry whose `kind` is `persona` or `prompt`. It
SHALL skip, without error, entries of any other kind.

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

The library is read-only to Knot. Importing SHALL NOT write to it.

#### Scenario: Personas and prompts are offered

- **WHEN** the index lists seven personas and three prompts
- **THEN** Knot offers all ten, each under its kind

#### Scenario: An unknown kind is skipped

- **WHEN** the index also lists items of kind `template`
- **THEN** Knot offers only the personas and prompts, and reports no error
  for the templates

#### Scenario: A newer index format

- **WHEN** the index's `format` is `2`
- **THEN** Knot offers nothing from the library and says this library needs
  a newer Knot

#### Scenario: A prompt keeps its variables

- **WHEN** Knot imports a prompt whose body contains `{{folder.name}}`
- **THEN** the imported prompt's text contains `{{folder.name}}` exactly as
  written

### Requirement: Imported library items keep their library identity

A persona or prompt imported from Knot-Library SHALL take the item's `id`
as its own. For this source, Knot already holds an item when it holds a
persona (for a persona item) or a prompt (for a prompt item) with that id,
in any state, including a soft-deleted built-in persona. A held item SHALL
be skipped, per "Import is additive and idempotent", whatever its name or
contents.

An imported persona SHALL be a user persona (`PersonaType::User`), the same
as one imported from a subagent definition: editable, and removed outright
when deleted. Once it is deleted, its id is no longer held, and the item
can be imported again.

Restore Defaults SHALL NOT read the library. It restores the built-in
personas shipped with Knot.

#### Scenario: A built-in persona is already held

- **WHEN** the library lists Kent Beck under the same id Knot ships it with
- **THEN** the item is skipped as already present, and no second Kent Beck
  appears

#### Scenario: A renamed copy is still the same item

- **WHEN** the user imports a library prompt, renames it in Knot, and opens
  the import again
- **THEN** the prompt is still skipped as already present

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

Knot SHALL fetch each item it imports from
`https://raw.githubusercontent.com/pilgrimagesoftware/Knot-Library/<commit>/<path>`,
using the index's `commit` and the entry's `path`. It SHALL fetch item files
only when the user confirms an import, and only for the items selected.

Before importing an item, Knot SHALL check that the downloaded bytes have
the entry's `size` and that their SHA-256 matches its `sha256`. An item that
fails to download, fails the check, or has an empty body SHALL be reported
as unreadable by its title, and SHALL NOT be imported. The other selected
items SHALL still be imported (see "Partial failure does not abandon the
import").

#### Scenario: A pinned download

- **WHEN** new content merges into the library after Knot read the index,
  and the user then imports an item
- **THEN** Knot imports the version the index describes

#### Scenario: Bytes that don't match

- **WHEN** one selected item's download has a different SHA-256 than its
  index entry
- **THEN** that item is reported as unreadable by title, and the other
  selected items are imported

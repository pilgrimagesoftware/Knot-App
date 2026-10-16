# data-import Specification

## Purpose

Defines how Knot brings definitions in from tools the user already has -
coding-agent subagent definitions that become personas, and a Skwad
installation's workspaces, agents, personas and bench templates - and the
rules every such import obeys.

## Requirements

### Requirement: Import is additive and idempotent

An import SHALL only add records. It SHALL NOT delete, overwrite, rename or
reorder anything already in Knot, and SHALL NOT modify the source it read
from.

A record that Knot already holds SHALL be skipped rather than duplicated or
merged, so running the same import twice leaves the second run with nothing
to do. What counts as "already held" is defined per source below.

An import SHALL report what it did: how many records were added and how many
were skipped as already present.

#### Scenario: Importing the same source twice adds nothing the second time

- **WHEN** the user runs an import, then runs the same import again with the
  source unchanged
- **THEN** the second run adds nothing, skips every record as already
  present, and reports that

#### Scenario: Import leaves the source alone

- **WHEN** any import completes
- **THEN** the files or preferences it read are unchanged

#### Scenario: An import never removes what the user has

- **WHEN** an import runs in a Knot that already holds personas, agents and
  workspaces
- **THEN** every one of them is still present afterwards, with its name,
  contents and ordering unchanged

### Requirement: Partial failure does not abandon the import

A source that cannot be read, or an individual record that cannot be
understood, SHALL NOT fail the whole import. The readable records SHALL be
imported and the unreadable ones reported by name, so one malformed file
cannot block the other sixteen.

#### Scenario: One malformed definition among many

- **WHEN** a source holds ten definitions and one of them cannot be parsed
- **THEN** the other nine are imported and the failing one is named in the
  result

#### Scenario: A source that is not installed

- **WHEN** an import is offered for a tool that is not installed, or whose
  definitions directory does not exist
- **THEN** that source contributes nothing, reports nothing to import, and
  raises no error

### Requirement: The user chooses what to import

An import SHALL present what it found and let the user select which records
to bring in before anything is written. An import SHALL NOT write on the
strength of being opened.

#### Scenario: Reviewing before importing

- **WHEN** the user opens an import and the source holds records
- **THEN** those records are listed for selection, and nothing is added until
  the user confirms

#### Scenario: Cancelling changes nothing

- **WHEN** the user opens an import and closes it without confirming
- **THEN** no record is added

### Requirement: Subagent definition provider registry

The system SHALL resolve a subagent-definition provider by coding-agent tool.
The registry SHALL know the tools `claude`, `codex`, `opencode` and `gemini`.
A reader exists for `claude`. For a tool the registry does not know, and for
one it knows but has no reader for, subagent import SHALL be reported as
unsupported and SHALL be a no-op.

A provider SHALL read only; it SHALL NOT create, modify or delete anything in
the tool it reads from.

The import surface SHALL offer only the tools that have a reader. A tool
whose format is not yet confirmed SHALL be absent from it rather than listed
and empty: "nothing to import" is indistinguishable from "not built yet", and
a user cannot tell which one they are looking at.

Codex, OpenCode and Gemini have no reader because their subagent format has
not been read from a real installation. None of the three holds subagent
definitions on the machine this change was written on: Codex keeps
`config.toml` and a `skills/` directory with no agents, OpenCode's
`opencode.jsonc` holds only a `$schema` key, and `~/.gemini` holds config,
history and plugins but no agents. A reader written from an inferred format
fails silently, which is worse than no reader at all, so each remains
unimplemented until its format can be confirmed against a populated install.

#### Scenario: Unsupported tool

- **WHEN** subagent definitions are requested for a tool with no provider
- **THEN** support is reported false and no read is attempted

#### Scenario: A tool with no confirmed format is not offered

- **WHEN** the user opens the import surface
- **THEN** only tools with a reader are listed, and Codex, OpenCode and
  Gemini are absent rather than shown holding nothing

### Requirement: Claude subagent source and format

The Claude provider SHALL read `*.md` files from `~/.claude/agents` and, when
a folder is given, from that folder's `.claude/agents`. Each file is YAML
frontmatter delimited by `---` lines followed by a body.

The provider SHALL take the frontmatter's `name` as the definition's name and
the body - everything after the closing delimiter, trimmed - as its
instructions. Frontmatter keys Knot has no equivalent for, including
`description`, `color` and `model`, SHALL be ignored rather than folded into
the instructions.

A file with no frontmatter, no `name` key, or an empty body SHALL be reported
as unreadable rather than imported under a guessed name.

#### Scenario: A definition becomes a named persona

- **WHEN** `~/.claude/agents/architect-review.md` has frontmatter naming it
  `architect-reviewer` and a body beginning "You are an expert software
  architect"
- **THEN** the definition is offered as `architect-reviewer` with that body
  as its instructions, and its `description`, `color` and `model` are dropped

#### Scenario: Project definitions are found alongside user ones

- **WHEN** an import is run for a folder that has its own `.claude/agents`
- **THEN** definitions from both `~/.claude/agents` and that folder are
  offered

#### Scenario: A file without a name is not guessed at

- **WHEN** a `.md` file in the definitions directory has no frontmatter
  `name`
- **THEN** it is reported as unreadable, and is not imported under its file
  name

### Requirement: Imported subagent definitions become user personas

Importing a subagent definition SHALL create a persona with the definition's
name, its instructions, type `user`, and state `enabled`, per the `personas`
capability's persona model. It SHALL NOT create a `system` persona, which
would make it behave like one of Knot's shipped defaults under "Restore
Defaults".

A definition whose name matches an existing persona's name SHALL be skipped
as already present. Subagent definitions carry no identifier of their own, so
the name is what identity there is.

#### Scenario: An imported definition is editable like any persona

- **WHEN** a definition is imported
- **THEN** the resulting persona is a `user` persona that can be edited and
  deleted like one the user wrote

#### Scenario: A definition that was already imported

- **WHEN** a definition named `architect-reviewer` is imported and a persona
  by that name already exists
- **THEN** it is skipped, and the existing persona's instructions are left as
  they are

### Requirement: Skwad source and format

The Skwad provider SHALL read the preferences domain `com.kochava.skwad`,
whose collection values are JSON documents stored as data: `savedWorkspacesData`,
`savedAgentsData`, `personasData` and `benchAgentsData`. Their record shapes
are the ones Knot's own settings already read.

An absent preferences domain, or an absent key within it, SHALL be treated as
nothing to import rather than an error - Skwad may not be installed.

#### Scenario: Skwad is not installed

- **WHEN** the Skwad import is opened on a machine with no
  `com.kochava.skwad` preferences
- **THEN** it reports that there is nothing to import, and raises no error

#### Scenario: Skwad preferences are read, not written

- **WHEN** a Skwad import completes
- **THEN** Skwad's preferences are byte-for-byte unchanged, and a running
  Skwad is unaffected

### Requirement: Importing a Skwad workspace brings what it needs

Importing a workspace SHALL bring the agents it holds, and for those agents
any persona they reference and any bench template for their folder, so an
imported workspace is usable rather than a name with broken references.

An agent whose persona cannot be found in the source SHALL be imported
without one rather than skipped.

Records carry Skwad's own identifiers. A workspace, agent or persona whose id
Knot already holds SHALL be skipped as already present, which is what makes
re-importing safe.

#### Scenario: A workspace arrives with its agents

- **WHEN** the user imports a Skwad workspace holding three agents, two of
  which use a persona
- **THEN** the workspace, its three agents and that persona are added, and
  the agents are in the workspace in their original order

#### Scenario: An agent whose persona is missing

- **WHEN** an imported agent references a persona id that is not in Skwad's
  personas
- **THEN** the agent is imported with no persona, and the missing reference
  is reported

#### Scenario: Re-importing a workspace already brought across

- **WHEN** the user imports a workspace that was imported before
- **THEN** it is skipped as already present, and the agents in Knot's copy -
  including any added since - are untouched

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

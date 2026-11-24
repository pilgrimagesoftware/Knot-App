# panel-slash-commands Specification

## Purpose

Lets the user discover and insert knot commands and agent skills by typing a
leading slash into the panel prompt input, completing text rather than
executing it.

## Requirements

### Requirement: Slash-triggered lookup

When the user types the agent's command character - `/` for every agent type
Knot runs - at the head of a line in the panel prompt input, the system SHALL
show a lookup popup listing the commands the registry offers, and the agent's
skills too when they share that character. An agent type whose skills have a
character of their own SHALL have its skills offered behind that character
instead. Each list SHALL filter to entries whose token or description matches
the text typed after its character.

The command lookup SHALL NOT be the popup's only occupant. The popup is shared
with the `@` file lookup and with a skill character's lookup, and the token
under the caret decides which one is active; the command lookup SHALL yield
when the caret sits in an `@` token or a skill token, and SHALL resume when the
caret returns to a command token. The token the lookup inserts SHALL be one the
composer's styling recognises, written with the character that opened it, so
an inserted command or skill is drawn as a token rather than as prose.

#### Scenario: Slash opens the lookup
- **WHEN** the user types `/` at the start of a line in the prompt input of an
  agent whose skills share the command character
- **THEN** a popup listing the available commands and skills appears above
  the input

#### Scenario: Typing filters the lookup
- **WHEN** the user types more characters after the `/`
- **THEN** the popup shows only the entries whose token or description matches
  the typed text

#### Scenario: No matches dismisses the lookup
- **WHEN** the text after the `/` matches no entry
- **THEN** the popup shows no entries and is dismissed

#### Scenario: The caret moves into an `@` token
- **WHEN** a slash lookup is open and the caret moves into an `@` token
  elsewhere in the buffer
- **THEN** the popup shows the file lookup instead, and the slash list is not
  shown alongside it

#### Scenario: An inserted command is styled as a token
- **WHEN** the lookup inserts a command into the buffer
- **THEN** the inserted range is drawn with the composer's token treatment

#### Scenario: A Codex agent's skills are behind `$`
- **WHEN** the agent is a Codex agent and the user types `$re` at the start
  of a word
- **THEN** a popup lists the agent's skills matching `re`, and no command or
  library prompt

#### Scenario: A Codex agent's slash offers commands, not skills
- **WHEN** the agent is a Codex agent and the user types `/` at the start of
  a line
- **THEN** the popup lists knot's commands and library prompts, and none of
  the agent's skills, which Codex would reject as `/name`

#### Scenario: A completed skill keeps its character
- **WHEN** the user completes a skill from a lookup opened with `$`
- **THEN** the buffer reads `$` followed by the skill's name

### Requirement: Lookup navigation
The lookup SHALL be navigable by keyboard: Up and Down move the selection,
Enter or Tab inserts the selected entry, and Esc dismisses the popup without
changing the input.

#### Scenario: Selecting and inserting
- **WHEN** the user selects an entry with the arrow keys and presses Enter or
  Tab
- **THEN** the entry is inserted and the popup closes

#### Scenario: Dismissing the lookup
- **WHEN** the user presses Esc while the lookup is open
- **THEN** the popup closes and the input text is unchanged

#### Scenario: Losing the token dismisses the lookup
- **WHEN** the user edits away the leading `/` token or removes focus from the
  input while the lookup is open
- **THEN** the popup closes

### Requirement: Inserting an entry completes text
Inserting an entry SHALL replace the partially typed slash token with the
entry's full token and leave the rest of the buffer untouched. Insertion
SHALL NOT execute the command or skill; the completed text is sent to the
agent like any other prompt.

#### Scenario: Token is replaced in place
- **WHEN** the user has typed `/sen` and inserts the `send` command
- **THEN** the input's text ends with the full `send` token in place of
  `/sen`, and any text already in the buffer outside the token is unchanged

#### Scenario: A completed prompt is sent as text
- **WHEN** the user inserts an entry and then sends the prompt
- **THEN** the prompt reaches the agent as ordinary text; nothing is executed
  by the panel

### Requirement: Lookup entry registry
The lookup SHALL draw its entries from a registry offering the panel's built-in
commands, the skills available to the selected agent, and the prompts in the
prompt library. Each entry SHALL carry a token and a description. A library
prompt's entry SHALL use the prompt's name as its token and a single-line
preview of its text as its description, and SHALL be distinguishable in the
popup from commands and skills. The registry SHALL be extensible without
changes to the lookup popup.

#### Scenario: Built-in commands are listed
- **WHEN** the slash lookup is open on a panel agent
- **THEN** the popup lists the built-in panel commands known to the app

#### Scenario: Agent skills are listed
- **WHEN** the selected agent has skills available
- **THEN** those skills appear in the slash lookup alongside the built-in
  commands

#### Scenario: A skill root that cannot be read is not an error
- **WHEN** a configured skill root is missing or unreadable
- **THEN** the lookup lists the entries it could read and treats the missing
  root as empty rather than failing the lookup

#### Scenario: Library prompts are listed
- **WHEN** the library holds a prompt named "Run the gate" and the user types
  `/gate`
- **THEN** the popup lists that prompt, marked as a library prompt

#### Scenario: A library change reaches an open composer
- **WHEN** a prompt is added in Settings while a panel composer is open
- **THEN** the next slash lookup in that composer lists the new prompt

### Requirement: Inserting a library prompt expands its text

Inserting a library prompt's entry SHALL replace the partially typed slash
token with the prompt's full text, line breaks included, with its variables
expanded for the selected agent (see `prompt-library` - Prompt variables),
and leave the rest of the buffer untouched.

A prompt that has user inputs SHALL first ask for them (see
`prompt-library` - Inserting a prompt with inputs asks for them). Insert
SHALL replace the token with the expanded text. Cancel SHALL leave the token
as typed, with the caret after it, and the lookup SHALL stay dismissed for
that token.

Expansion SHALL NOT block the window on I/O (resolving `branch` reads the
repository). The composer SHALL stay responsive while it resolves, and the
text SHALL be inserted at the token's position once it has.

Choosing a library prompt's entry SHALL dismiss the lookup, as choosing any
entry does. That dismissal SHALL NOT discard the insertion. An insertion
SHALL be discarded only if the composer's text changes before it lands: the
user edits the token or anything else in the buffer while the expansion
resolves or while the inputs sheet is open.

The expanded text SHALL be ordinary composer text: the user can edit it
before sending, and nothing is sent by the insertion itself.

This differs from "Inserting an entry completes text", which inserts a
command's or skill's token; a library prompt has no token the agent would
recognise, so its text is what is inserted.

#### Scenario: A prompt expands in place
- **WHEN** the buffer holds `note:` on one line and `/gat` at the start of
  the next, and the user inserts the library prompt "Run the gate" whose
  text is `make, then commit`
- **THEN** the buffer holds `note:` on the first line and
  `make, then commit` on the second

#### Scenario: Variables are expanded on insertion
- **WHEN** the user inserts a library prompt whose text is
  `Review {{folder.name}}` into the composer of an agent in `/src/knot`
- **THEN** the buffer holds `Review knot`

#### Scenario: Inputs are asked for before the token is replaced
- **WHEN** the buffer holds `/rev` and the user inserts the library prompt
  "Review" whose text is `Review {{user_input:PR}}`, enters `#466` in the
  sheet and presses Insert
- **THEN** the buffer holds `Review #466`

#### Scenario: Cancelling the inputs sheet keeps the token
- **WHEN** the buffer holds `/rev`, the user inserts a library prompt with
  inputs and cancels the sheet
- **THEN** the buffer still holds `/rev` and the composer has focus

#### Scenario: A stale expansion is dropped
- **WHEN** the user inserts a library prompt and edits the slash token before
  its expansion has resolved
- **THEN** the expansion is discarded and the buffer keeps the user's edit

#### Scenario: Expansion does not send
- **WHEN** the user inserts a library prompt
- **THEN** nothing is sent to the agent until the user sends the composer

## MODIFIED Requirements

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

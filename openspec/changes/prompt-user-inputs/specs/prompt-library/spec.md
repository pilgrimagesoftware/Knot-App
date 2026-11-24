## ADDED Requirements

### Requirement: User inputs

Prompt text SHALL support user inputs: values the user types when the
prompt is inserted into a composer. An input SHALL be written in the prompt
variables' braces as `user_input:` followed by a label, with an optional
default after the first `|`:

- `{{user_input:What is your quest?}}`: label `What is your quest?`, no
  default
- `{{user_input:Ticket|KNOT-1}}`: label `Ticket`, default `KNOT-1`
- `{{ user_input: Focus | }}`: label `Focus`, default empty

The `user_input:` keyword SHALL be matched case-sensitively after the
whitespace inside the braces is trimmed. The label SHALL be the text between
`user_input:` and the first `|` (or the closing braces), with surrounding
whitespace trimmed. The default SHALL be the rest, up to the closing braces,
with surrounding whitespace trimmed. A label SHALL NOT be empty after
trimming. An input with a blank label SHALL be left exactly as typed, like
an unknown variable. A label cannot contain `|` or `}}`, and a default cannot
contain `}}`.

The braces, whitespace, escaping and unclosed-brace rules SHALL be the
prompt variables' rules (see "Unknown variables and escaping"): `\{{` before
`user_input:` writes it literally and asks for nothing.

A prompt's inputs SHALL be its distinct labels in the order each first
appears. Labels SHALL be compared exactly, after trimming, and
case-sensitively. Every occurrence of a label SHALL expand to the same value.
When occurrences of one label give different defaults, the first
occurrence's default SHALL be used.

An input's value SHALL be inserted literally, like a variable's value. A
value that contains `{{` SHALL NOT be expanded again.

Inputs SHALL be expanded when the text is used, never when it is stored. The
library keeps them as written.

The Swift app has no user inputs.

#### Scenario: An input is listed once per label

- **WHEN** the text is `Fix {{user_input:Ticket}} and close {{user_input:Ticket}} on {{user_input:Branch|main}}`
- **THEN** the prompt's inputs are `Ticket` with no default, then `Branch`
  with default `main`

#### Scenario: A repeated label uses one value

- **WHEN** the text is `{{user_input:Name}} / {{user_input:Name}}` and the
  user enters `knot`
- **THEN** the expanded text is `knot / knot`

#### Scenario: Inputs and variables expand together

- **WHEN** the text is `Review {{user_input:PR}} on {{branch}}`, the agent's
  folder is on branch `main`, and the user enters `#466`
- **THEN** the expanded text is `Review #466 on main`

#### Scenario: A value is not expanded twice

- **WHEN** the user enters `{{folder}}` for an input
- **THEN** the expanded text holds the literal `{{folder}}`

#### Scenario: A blank label is left as typed

- **WHEN** the text holds `{{user_input: }}`
- **THEN** the expanded text holds `{{user_input: }}`, and the prompt has no
  inputs

#### Scenario: An escaped input is literal

- **WHEN** the text holds `\{{user_input:Name}}`
- **THEN** the prompt has no inputs, and the expanded text holds
  `{{user_input:Name}}`

### Requirement: Inserting a prompt with inputs asks for them

Inserting into a composer a library prompt that has one or more inputs
SHALL first open an inputs sheet. Inserting one without inputs SHALL NOT
open a sheet. Both the Insert Prompt list and the `/` lookup SHALL insert
this way.

The sheet SHALL be its own window, titled with the prompt's name, and SHALL
show one single-line field per input, in the prompt's input order. Each field
SHALL be labelled with its input's label, SHALL be prefilled with its
default (empty when it has none), and SHALL start empty each time the sheet
opens apart from that default. The sheet SHALL have Cancel and Insert
buttons.

- The first field SHALL have focus when the sheet opens, with its prefilled
  text selected.
- Tab and Shift-Tab SHALL move between fields.
- Return SHALL move to the next field, and in the last field SHALL insert.
- Cmd-Return SHALL insert from any field.
- Esc, Cancel and closing the window SHALL cancel.

An empty value SHALL be allowed. A field the user empties SHALL expand to
the empty string, not to its default.

Insert SHALL close the sheet, expand the prompt's variables and inputs, and
insert the result as described in "Choosing a prompt from the Insert Prompt
list" or `panel-slash-commands` "Inserting a library prompt expands its
text". Cancel SHALL close the sheet and insert nothing. The composer's text
and selection SHALL then be as they were before the prompt was chosen, and
focus SHALL return to the composer.

`{{clipboard}}` SHALL be captured when the prompt is chosen, before the
sheet opens (see "The clipboard is captured when a prompt is chosen").

If the composer's text changes while the sheet is open, Insert SHALL insert
nothing, as for an edit made while the expansion resolves. Only one inputs
sheet SHALL be open per composer. Choosing another prompt while one is open
SHALL bring the open sheet to the front instead of opening a second sheet.

#### Scenario: One sheet asks for every input

- **WHEN** the user chooses the prompt "Quest", whose text is
  `{{user_input:Name}} seeks {{user_input:What is your quest?}}`, from the
  Insert Prompt list
- **THEN** a sheet titled "Quest" opens with a `Name` field and a
  `What is your quest?` field, in that order, and the first has focus

#### Scenario: Insert substitutes the values

- **WHEN** in that sheet the user enters `Arthur` and `the Grail` and
  presses Insert
- **THEN** the sheet closes, and the composer receives
  `Arthur seeks the Grail` at its caret, and nothing is sent

#### Scenario: Cancel inserts nothing

- **WHEN** the composer holds `keep me`, the user chooses a prompt with
  inputs, and then presses Esc in the sheet
- **THEN** the sheet closes, the composer still holds `keep me` with its
  selection unchanged, and the composer has focus

#### Scenario: A prompt without inputs inserts directly

- **WHEN** the user chooses a prompt whose text is `review {{branch}}`
- **THEN** no sheet opens, and the expanded text is inserted

#### Scenario: A default prefills its field

- **WHEN** the user chooses a prompt whose text is
  `{{user_input:Branch|develop}}`
- **THEN** the sheet's `Branch` field holds `develop`, selected

#### Scenario: An emptied default expands to nothing

- **WHEN** the user clears the prefilled `develop` and presses Insert
- **THEN** the input expands to the empty string

#### Scenario: Editing the composer while the sheet is open

- **WHEN** the sheet is open and the user types in the composer, then
  presses Insert
- **THEN** nothing is inserted, and the user's typing is kept

### Requirement: Startup prompts never ask for inputs

A startup prompt is sent without the user choosing it, on a fresh session,
scheduled run or restart, and nothing SHALL wait on the user for it. In a
startup prompt, an input SHALL expand to its default, or to the empty string
when it has none. No sheet SHALL open, and the launch SHALL NOT wait for
one.

The agent editor's custom startup-prompt field SHALL warn about each input
that has no default, saying it is empty in startup prompts. The warning
SHALL NOT block save. An agent whose startup prompt references a library
prompt with inputs SHALL be treated the same way: inputs with no default
expand to the empty string, and the launch proceeds.

#### Scenario: A startup prompt uses the default

- **WHEN** an agent's startup prompt is `Work on {{user_input:Branch|develop}}`
  and it starts a fresh session
- **THEN** the agent is sent `Work on develop` and no sheet opens

#### Scenario: An input without a default is empty at startup

- **WHEN** an agent's startup prompt is `Ticket: {{user_input:Ticket}}`
  and a scheduled run starts it
- **THEN** the agent is sent `Ticket: ` and the run does not wait

#### Scenario: Startup prompt editor warns

- **WHEN** the user types `{{user_input:Ticket}}` into the agent editor's
  custom startup-prompt field
- **THEN** a warning says `Ticket` is empty in startup prompts, and save
  remains enabled

## MODIFIED Requirements

### Requirement: Unknown variables and escaping

A `{{...}}` whose name is not one of the prompt variables, and is not a user
input (see "User inputs"), SHALL be left exactly as typed, braces included,
and SHALL NOT prevent the prompt from being sent or inserted. A user input
with a blank label counts as unknown. Anything that edits prompt text - the
Prompts window's editor and the agent editor's custom startup-prompt field -
SHALL flag each unknown name as a warning, without blocking save, so a
misspelling is caught before it reaches an agent. A blank-label input SHALL
be flagged as a malformed input rather than as an unknown name.

A backslash immediately before `{{` SHALL write a literal `{{` and suppress
expansion of what follows it; the backslash itself SHALL NOT appear in the
expanded text. A `{{` with no closing `}}` SHALL be left as typed.

#### Scenario: An unknown name is left as typed

- **WHEN** the text holds `{{issue}}`
- **THEN** the expanded text holds `{{issue}}`, unchanged

#### Scenario: A misspelling is flagged but saveable

- **WHEN** the user types `{{foldr}}` into the prompt editor
- **THEN** the editor shows a warning naming `foldr` as unknown, and save
  remains enabled

#### Scenario: A user input is not unknown

- **WHEN** the user types `{{user_input:Ticket}}` into the prompt editor
- **THEN** no unknown-variable warning is shown

#### Scenario: A misspelled keyword is unknown

- **WHEN** the user types `{{user-input:Ticket}}` into the prompt editor
- **THEN** the editor warns that `user-input:Ticket` is unknown

#### Scenario: An escaped variable is written literally

- **WHEN** the text holds `\{{folder}}`
- **THEN** the expanded text holds `{{folder}}`, with no backslash and no
  path

#### Scenario: An unclosed brace is left alone

- **WHEN** the text holds `use {{ carefully`
- **THEN** the expanded text is `use {{ carefully`

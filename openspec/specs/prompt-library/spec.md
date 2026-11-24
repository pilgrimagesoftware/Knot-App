# prompt-library Specification

## Purpose
Defines the prompt library - named prompts the user writes once and reuses as
agents' startup prompts and from the panel's `/` lookup - the startup prompt
an agent or bench entry carries, by library reference or as its own text,
and the variables prompt text can use to adapt to the agent it is sent to.

## Requirements

### Requirement: Prompt model

A library prompt SHALL have an id, a name and a text. The name SHALL be
non-empty after trimming surrounding whitespace, and the text SHALL be
non-empty after trimming. Names SHALL NOT be required to be unique; the id is
what identifies a prompt.

The text MAY span several lines, and SHALL be stored exactly as entered.

The Swift app has no prompt library; this capability is new to the Rust port.

#### Scenario: A blank name is refused

- **WHEN** a prompt is added whose name is only whitespace
- **THEN** the add is refused and the library is unchanged

#### Scenario: Multi-line text is stored as entered

- **WHEN** a prompt is added whose text holds three lines
- **THEN** reloading the library yields the same three lines, unchanged

### Requirement: Library operations

The library SHALL support adding, updating (name and text), duplicating and
removing a prompt by id. It SHALL keep prompts in the order they were added,
except a duplicate, which follows its original. Each operation SHALL persist
the library document before it reports success.

Removing a prompt SHALL NOT rewrite any agent or bench entry that references
it; such a reference then resolves as described in "A reference to a removed
prompt".

#### Scenario: Updating a prompt changes every agent that references it

- **WHEN** two agents reference library prompt P as their startup prompt and
  P's text is updated
- **THEN** each agent's next fresh session is sent P's new text

#### Scenario: Removing a prompt leaves referencing agents untouched

- **WHEN** a prompt referenced by an agent is removed
- **THEN** the agents document is not rewritten by the removal

### Requirement: Startup prompt forms

An agent's or bench entry's startup prompt SHALL be one of: none, a reference
to a library prompt by id, or custom text held on the agent or entry itself.
Custom text SHALL follow the same non-empty rule as a library prompt's text; a
custom startup prompt whose text is blank SHALL be stored as none.

Resolving a startup prompt SHALL yield the text to send: nothing for none, the
referenced prompt's current text for a reference, and the held text for custom
text - in each case with its variables expanded for the agent it is sent to
(see "Prompt variables").

#### Scenario: A reference resolves to the prompt's current text

- **WHEN** an agent's startup prompt references library prompt P
- **THEN** resolving it yields P's text as it stands at resolution time

#### Scenario: Blank custom text is none

- **WHEN** the user saves an agent with custom startup-prompt text that is
  only whitespace
- **THEN** the agent is stored with no startup prompt

### Requirement: A reference to a removed prompt

A startup prompt that references an id no longer in the library SHALL resolve
to nothing: no startup prompt is sent, and the launch otherwise proceeds
normally. The reference SHALL be kept, not cleared, so that anything showing
the agent's startup prompt can report it as missing.

#### Scenario: Launch with a dangling reference

- **WHEN** an agent whose startup prompt references a removed prompt starts a
  fresh session
- **THEN** the initialization prompt is sent, no startup prompt follows it,
  and no error is shown in the conversation

### Requirement: Prompt variables

Prompt text - a library prompt's and a custom startup prompt's alike - SHALL
support the following variables, each written as its name between `{{` and
`}}`, with optional whitespace inside the braces (`{{ folder }}` is
`{{folder}}`):

| Variable | Expands to |
|---|---|
| `agent.name` | the agent's name |
| `agent.id` | the agent's knot agent ID |
| `agent.type` | the agent's type, e.g. `claude` |
| `folder` | the agent's folder, as an absolute path |
| `folder.name` | the last path component of the agent's folder |
| `workspace` | the name of the workspace the agent belongs to; empty if none |
| `branch` | the current branch of the agent's folder; the short commit hash when HEAD is detached; empty when the folder is not in a git repository |
| `date` | today's date in the local time zone, as `YYYY-MM-DD` |
| `clipboard` | the text on the system clipboard when the prompt is inserted into a composer; empty when the clipboard holds no text, and always empty in a startup prompt |

The set is closed: these are the only names that expand. Names SHALL be
matched case-sensitively.

Expansion SHALL happen when the text is used, never when it is stored: the
library and every startup prompt keep the variables as written, so one
prompt serves every agent, and `branch`, `date` and `clipboard` are current
each time.

Values SHALL be inserted literally; a value that itself contains `{{` SHALL
NOT be expanded again.

The Swift app has no prompt variables.

#### Scenario: Variables expand for the receiving agent

- **WHEN** the prompt `Work in {{folder.name}} on {{branch}}` is sent to an
  agent in `/src/knot` whose folder is on branch `471-restart`
- **THEN** the text sent is `Work in knot on 471-restart`

#### Scenario: One library prompt adapts per agent

- **WHEN** two agents in different folders both reference a library prompt
  that uses `{{folder}}`
- **THEN** each is sent its own folder's path

#### Scenario: Whitespace inside the braces is allowed

- **WHEN** the text holds `{{ agent.name }}` and the agent is named "Knot 3"
- **THEN** it expands to `Knot 3`

#### Scenario: Branch outside a repository is empty

- **WHEN** the text uses `{{branch}}` and the agent's folder is not in a git
  repository
- **THEN** `{{branch}}` expands to the empty string

#### Scenario: A value is not expanded twice

- **WHEN** an agent is named `{{date}}` and the text is `{{agent.name}}`
- **THEN** the expanded text is the literal `{{date}}`

#### Scenario: Clipboard text is inserted

- **WHEN** the clipboard holds `panic at main.rs:12` and the user inserts the
  library prompt `Explain: {{clipboard}}` into a composer
- **THEN** the composer receives `Explain: panic at main.rs:12`

#### Scenario: Clipboard braces are not expanded

- **WHEN** the clipboard holds `{{branch}}` and an inserted prompt uses
  `{{clipboard}}`
- **THEN** the literal `{{branch}}` is inserted

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

### Requirement: The clipboard is captured when a prompt is chosen

`{{clipboard}}` SHALL be read from the system clipboard once, when the user
chooses the prompt to insert, before any other variable is resolved. Later
clipboard changes SHALL NOT affect the inserted text. Only text SHALL be
used. A clipboard holding no text SHALL expand to the empty string.

#### Scenario: Clipboard holds an image

- **WHEN** the clipboard holds only an image and the user inserts a prompt
  using `{{clipboard}}`
- **THEN** `{{clipboard}}` expands to the empty string

#### Scenario: Clipboard changes during expansion

- **WHEN** the user chooses a prompt using `{{clipboard}}` while the
  clipboard holds `one`, then copies `two` before expansion completes
- **THEN** the inserted text holds `one`

### Requirement: Startup prompts never read the clipboard

A startup prompt is sent without the user choosing it. In a startup prompt,
`{{clipboard}}` SHALL expand to the empty string, and the clipboard SHALL NOT
be read. The agent editor's custom startup-prompt field SHALL warn that
`{{clipboard}}` is always empty there, and the warning SHALL NOT block save.

#### Scenario: Startup prompt does not read the clipboard

- **WHEN** an agent's custom startup prompt is `Context: {{clipboard}}` and
  the clipboard holds a secret
- **THEN** the agent is sent `Context: ` and the secret is not sent

#### Scenario: Startup prompt editor warns

- **WHEN** the user types `{{clipboard}}` into the agent editor's custom
  startup-prompt field
- **THEN** a warning says the clipboard is empty in startup prompts, and save
  remains enabled

### Requirement: The composer's Insert Prompt list

A panel agent's composer, but not a shell companion's, SHALL offer an Insert
Prompt control in its control bar. It SHALL list the library's prompts in
library order, by name with a one-line preview, filtered as the user types
to those whose name or text contains the typed text. Keys SHALL behave as in
the `/` lookup. An empty library SHALL offer to open the Prompts window.

#### Scenario: Filtering

- **WHEN** the library holds "Review" and "Run the gate" and the user types
  `gate` in the picker
- **THEN** only "Run the gate" is listed

#### Scenario: Empty library

- **WHEN** the library holds no prompts and the user opens Insert Prompt
- **THEN** the list says there are no saved prompts and offers to open the
  Prompts window

#### Scenario: Dismissing changes nothing

- **WHEN** the user opens Insert Prompt and presses Esc
- **THEN** the composer's text and selection are unchanged and it has focus

### Requirement: Choosing a prompt from the Insert Prompt list

Choosing a prompt SHALL insert its text at the composer's caret, or replace
the selection if there is one, with variables expanded for that composer's
agent. The rest of the buffer SHALL be left untouched. Expansion SHALL follow
"Inserting a library prompt expands its text" (`panel-slash-commands`).
Nothing SHALL be sent, and focus SHALL return to the composer with the caret
after the inserted text.

#### Scenario: Insert at the caret

- **WHEN** the composer holds `Please ` with the caret at the end, and the
  user opens Insert Prompt and chooses "Review" whose text is
  `review {{branch}}` on branch `main`
- **THEN** the composer holds `Please review main`, the caret is at the end,
  and nothing is sent

#### Scenario: Replace a selection

- **WHEN** `TODO` is selected in the composer and the user chooses a prompt
  whose text is `run the tests`
- **THEN** `TODO` is replaced by `run the tests`

### Requirement: Adding a prompt from the Insert Prompt list

The Insert Prompt list SHALL offer to add a prompt in two places: an add
button beside its search field, and a "New Prompt…" item at the foot of the
list, shown whatever the list holds. Each SHALL close the list and open the
prompt editor on a blank prompt with the caret in its name, leaving the
composer unchanged.

#### Scenario: Adding a prompt from the list

- **WHEN** the user opens Insert Prompt and clicks the add button or the
  "New Prompt…" item
- **THEN** the prompt editor opens on a blank prompt with the caret in its
  name, and the list is closed

### Requirement: Editing a prompt from the Insert Prompt list

Each prompt the Insert Prompt list shows SHALL carry an edit button that
closes the list and opens the prompt editor on that prompt with the caret in
its text. It SHALL NOT insert the prompt, change the composer, or send
anything.

#### Scenario: Editing a listed prompt

- **WHEN** the composer holds `keep me` and the user opens Insert Prompt and
  clicks the edit button on "Run the gate"
- **THEN** the prompt editor opens on "Run the gate" with the caret in its
  text, the list is closed, and the composer still holds `keep me`

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
A label's default SHALL be the first non-empty default any of its
occurrences gives, in text order, whether or not the first occurrence gives
one. An empty default (`Label|`) SHALL count as no default.

An input's value SHALL be inserted literally, like a variable's value. A
value that contains `{{` SHALL NOT be expanded again.

Inputs SHALL be expanded when the text is used, never when it is stored. The
library keeps them as written.

The Swift app has no user inputs.

#### Scenario: An input is listed once per label

- **WHEN** the text is `Fix {{user_input:Ticket}} and close {{user_input:Ticket}} on {{user_input:Branch|main}}`
- **THEN** the prompt's inputs are `Ticket` with no default, then `Branch`
  with default `main`

#### Scenario: A later occurrence can give the default

- **WHEN** the text is `{{user_input:T}} then {{user_input:T|x}} then {{user_input:T|y}}`
- **THEN** the prompt has one input, `T`, with default `x`

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
and selection SHALL then be as they were before the prompt was chosen. On
both Insert and Cancel the workspace window SHALL be activated and its
composer focused.

`{{clipboard}}` SHALL be captured when the prompt is chosen, before the
sheet opens (see "The clipboard is captured when a prompt is chosen").

If the composer's text changes while the sheet is open, Insert SHALL insert
nothing, as for an edit made while the expansion resolves.

Only one inputs sheet SHALL be open per composer. Choosing any prompt with
inputs, the same one or another, while a sheet is open SHALL bring the open
sheet to the front and drop the new choice: nothing is inserted for it and
the open sheet keeps its prompt and values. Choosing a prompt without inputs
while a sheet is open SHALL insert as usual. The open sheet's own Insert
then finds the composer changed and inserts nothing.

When anything takes away the composer that opened a sheet, the sheet SHALL
close and insert nothing: removing, deactivating or moving its agent to
another window, or closing the workspace window holding that composer. The
close SHALL NOT wait on an unrelated redraw. Restarting the agent keeps its
composer, so its sheet SHALL stay open.

#### Scenario: A second choice brings the open sheet forward

- **WHEN** the sheet for prompt "Quest" is open and the user chooses the
  prompt "Review", which has inputs, from the same composer
- **THEN** no second sheet opens, the "Quest" sheet comes to the front with
  its values unchanged, and nothing is inserted for "Review"

#### Scenario: Removing the agent closes its sheet

- **WHEN** an inputs sheet is open for agent Ada's composer and Ada is
  removed
- **THEN** the sheet closes and nothing is inserted

#### Scenario: Deactivating the agent closes its sheet

- **WHEN** an inputs sheet is open for agent Ada's composer and Ada is
  deactivated
- **THEN** the sheet closes and nothing is inserted

#### Scenario: Closing the workspace window closes its sheets

- **WHEN** an inputs sheet is open and the workspace window holding its
  composer closes
- **THEN** the sheet closes and nothing is inserted

#### Scenario: Cancel returns focus to the composer

- **WHEN** the user presses Cancel in an inputs sheet
- **THEN** the workspace window is active and the composer that opened the
  sheet has focus

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

### Requirement: At most five inputs are asked for

The inputs sheet SHALL show at most five fields: the prompt's first five
inputs, in input order. Each input beyond the fifth SHALL expand to its
default, or to the empty string when it has none, as in a startup prompt.
When any inputs are left out, the sheet SHALL show a note under its fields
saying how many inputs will use their defaults, in the singular for one and
the plural otherwise.

The Prompts window's editor SHALL warn when a prompt has more than five
inputs, naming the limit. The warning SHALL NOT block save.

#### Scenario: Five inputs are all asked for

- **WHEN** the user chooses a prompt with exactly five distinct inputs
- **THEN** the sheet shows five fields and no note

#### Scenario: A sixth input uses its default

- **WHEN** the user chooses a prompt whose inputs are `A` to `E`, then
  `F|six`, enters values for `A` to `E` and presses Insert
- **THEN** the sheet showed five fields and a note saying one more input
  will use its default, and `F` expands to `six`

#### Scenario: The editor warns past five inputs

- **WHEN** the user types a prompt with six distinct inputs into the
  Prompts window's editor
- **THEN** a warning says only the first five inputs are asked for, and save
  remains enabled

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

### Requirement: Duplicating a prompt

Duplicating a prompt SHALL add a prompt with a new id and every other field
of the original - text, description and any field added later - copied as it
stands, placed right after the original. Its name SHALL be the original's
followed by " copy", or by " copy 2", " copy 3" and so on when that name is
taken, using the catalogue's wording. An unknown id SHALL change nothing.

#### Scenario: A duplicate follows its original

- **WHEN** the library holds "Review" then "Gate", and "Review" is duplicated
- **THEN** the library holds "Review", "Review copy", "Gate" in that order,
  and "Review copy" has a new id and "Review"'s text and description

#### Scenario: A taken copy name is numbered

- **WHEN** the library holds "Review" and "Review copy", and "Review" is
  duplicated
- **THEN** the new prompt is named "Review copy 2"

#### Scenario: A duplicate survives a reload

- **WHEN** a prompt is duplicated and the library is reloaded
- **THEN** the copy is there, after the original

#### Scenario: Duplicating an unknown id writes nothing

- **WHEN** an id that is not in the library is duplicated
- **THEN** the library is unchanged and no document is written

### Requirement: Duplicating a prompt from the Insert Prompt list

Each prompt the Insert Prompt list shows SHALL carry a duplicate button beside
its edit button. It SHALL close the list, duplicate the prompt (see "Library
operations"), and open the prompt editor on the copy with the caret in its
name. It SHALL NOT insert anything, change the composer, or send anything.

#### Scenario: Duplicating a listed prompt

- **WHEN** the composer holds `keep me` and the user opens Insert Prompt and
  clicks the duplicate button on "Review"
- **THEN** a copy of "Review" is placed right after it, the prompt editor
  opens on the copy with the caret in its name, the list is closed, and the
  composer still holds `keep me`

# library-windows Specification

## Purpose
Defines the Personas, Prompts and Bench windows in the `knot` app: how each
opens, and what its list and editor do. They were tabs of the settings window
until #20 moved them out (`settings-ui`).

## Requirements

### Requirement: Library windows are opened from the Window menu

Personas, Prompts and Bench SHALL each have a window of its own, opened from
the Window menu (see `app-menu`). Each SHALL be a single window: opening one
that is already open SHALL bring it to the front rather than open a second
(see `window-lifecycle`). Each window SHALL be titled with its library's name
("Personas", "Prompts", "Bench").

These windows held tabs of the settings window until the libraries outgrew
it. They manage content rather than preferences, and a separate window lets
two libraries be open side by side.

#### Scenario: Opening the Personas window

- **WHEN** the user chooses Window > Personas
- **THEN** a window titled "Personas" opens showing the persona list

#### Scenario: Two libraries side by side

- **WHEN** the Prompts window is open and the user chooses Window > Bench
- **THEN** the Bench window opens and the Prompts window stays open

### Requirement: Personas window

The Personas window SHALL list every non-deleted persona (name and a
truncated instructions preview) with per-row edit and delete actions, an
"Add Persona…" action, and a "Restore Defaults" action gated behind a
confirmation dialog. A persona assigned to agents SHALL name them on its row,
each name a link that opens that agent's workspace window with the agent
selected, or raises the window if it is open.

#### Scenario: Empty list shows a message, not nothing

- **WHEN** no personas exist
- **THEN** the Personas window shows "No personas defined" instead of an
  empty list

#### Scenario: Adding a persona

- **WHEN** the user clicks "Add Persona…", enters a name and instructions,
  and saves
- **THEN** a new persona is added via `add_persona` and appears in the list
  immediately

#### Scenario: Editing a persona

- **WHEN** the user clicks the edit action on an existing persona, changes
  its instructions, and saves
- **THEN** `update_persona` is called with that persona's id and the list
  reflects the new instructions

#### Scenario: Canceling an edit discards changes

- **WHEN** the user opens the editor for a persona, changes a field, and
  cancels instead of saving
- **THEN** the persona's stored name and instructions are unchanged

#### Scenario: Deleting a persona

- **WHEN** the user clicks the delete action on a persona no agent is
  assigned and confirms the dialog that names it
- **THEN** `remove_persona` is called for that persona's id

#### Scenario: A persona in use cannot be deleted

- **WHEN** a persona is assigned to agents Alpha and Beta
- **THEN** its delete action is disabled, its tooltip says it is in use by
  Alpha and Beta, and its row lists Alpha and Beta as "Used by"

#### Scenario: Going to an agent that uses a persona

- **WHEN** the user clicks an agent's name on a persona's "Used by" line
- **THEN** that agent's workspace window opens, or comes forward, with the
  agent selected

#### Scenario: Restoring defaults requires confirmation

- **WHEN** the user clicks "Restore Defaults"
- **THEN** a confirmation dialog appears before `restore_default_personas`
  is called; canceling the dialog calls nothing

### Requirement: Prompts window

The Prompts window SHALL list every library prompt
(name and a truncated, single-line preview of its text) in library order,
with per-row edit and delete actions and an "Add Prompt…" action. Adding and
editing SHALL use one editor with a name field and a multi-line text field;
its save action SHALL be disabled while either field is blank.

The editor SHALL list the prompt variables (see `prompt-library` - Prompt
variables) with what each expands to, and choosing one SHALL insert it at the
text field's caret. The list SHALL end with a user-input entry (see
`prompt-library` - User inputs). Choosing it SHALL insert
`{{user_input:}}` with the caret before the closing braces, ready for a
label. The editor SHALL flag unknown variable names, malformed user inputs,
and more than five inputs as warnings without disabling save (see
`prompt-library` - Unknown variables and escaping, and At most five inputs
are asked for).

Deleting a prompt that no agent or bench entry references SHALL happen
immediately. Deleting one that is referenced SHALL first ask for
confirmation, and the confirmation SHALL say how many agents and bench
entries reference it, since each of them will launch without a startup
prompt afterwards.

#### Scenario: Empty library shows a message

- **WHEN** the library holds no prompts
- **THEN** the Prompts window shows a message saying no prompts are defined
  instead of an empty list

#### Scenario: Adding a prompt

- **WHEN** the user clicks "Add Prompt…", enters a name and text, and saves
- **THEN** the prompt is added to the library and appears at the end of the
  list immediately

#### Scenario: Save is disabled for a blank field

- **WHEN** the prompt editor's name is filled in and its text is blank
- **THEN** the save action is disabled

#### Scenario: Inserting a variable from the list

- **WHEN** the user places the caret in the text field and chooses
  `folder.name` from the variable list
- **THEN** `{{folder.name}}` is inserted at the caret

#### Scenario: Inserting a user input from the list

- **WHEN** the user places the caret in the text field and chooses the
  user-input entry
- **THEN** `{{user_input:}}` is inserted, with the caret after `user_input:`

#### Scenario: A blank-label input is flagged but saveable

- **WHEN** the prompt's text holds `{{user_input:}}` and the user saves
- **THEN** the editor shows a malformed-input warning, and the save succeeds

#### Scenario: More than five inputs are flagged but saveable

- **WHEN** the prompt's text has six distinct inputs and the user saves
- **THEN** the editor shows the five-input warning, and the save succeeds

#### Scenario: Canceling an edit discards changes

- **WHEN** the user opens the editor for a prompt, changes its text, and
  cancels
- **THEN** the prompt's stored name and text are unchanged

#### Scenario: Deleting a referenced prompt asks first

- **WHEN** the user deletes a prompt that one agent and one bench entry
  reference
- **THEN** a confirmation names one agent and one bench entry, and canceling
  it leaves the prompt in the library

#### Scenario: Deleting an unreferenced prompt does not ask

- **WHEN** the user deletes a prompt nothing references
- **THEN** it is removed immediately, with no confirmation

### Requirement: Bench window

The Bench window SHALL list every bench entry (avatar,
name, folder's last path component, and its startup prompt's name or "Custom"
or nothing) with per-row edit and remove actions. Editing SHALL change the
entry's name and startup prompt only; the other template fields come from the
agent the entry was saved from and are changed by saving that agent to the
bench again.

Removing an entry SHALL ask for confirmation naming the entry, matching the
Swift app's bench dropdown.

The Swift app edits the bench only from the New Agent button's dropdown, and
there only removes entries. The Rust port keeps that dropdown (see
`agent-list-ui` - The New Agent button opens the bench) and adds this window for
editing, which the dropdown has no room for.

#### Scenario: Empty bench shows how to fill it

- **WHEN** the bench holds no entries
- **THEN** the Bench window shows a message saying an agent is added from its
  row menu's Save to Bench or Bench Agent

#### Scenario: Renaming a bench entry

- **WHEN** the user edits a bench entry's name and saves
- **THEN** the stored entry carries the new name and its other fields are
  unchanged

#### Scenario: Removing a bench entry asks first

- **WHEN** the user removes a bench entry and cancels the confirmation
- **THEN** the entry is still on the bench

### Requirement: The Personas and Prompts windows open the library import

The Personas window and the Prompts window SHALL each have an "Import from
Library…" action beside "Add". Choosing it SHALL open the Import window, or
bring it to the front if it is already open (see `import-ui`), where the
Library section lists what can be imported.

The Bench window SHALL NOT have this action until the library format has a
bench kind.

#### Scenario: Importing from the Personas window

- **WHEN** the user chooses "Import from Library…" in the Personas window
- **THEN** the Import window opens, and its Library section reads
  Knot-Library and lists its personas and prompts

#### Scenario: Imported personas appear in the open window

- **WHEN** the Personas window is open and the user imports a library
  persona
- **THEN** the persona appears in the Personas window's list without
  reopening it

### Requirement: A persona or prompt can be submitted to Knot-Library

Each row in the Personas and Prompts windows SHALL offer to submit its item
to Knot-Library. Submitting SHALL ask for a title, a one-line description and
optional tags, and SHALL NOT proceed until they meet Knot-Library's
contribution rules.

#### Scenario: The form is checked before anything opens

- **WHEN** the description is empty, or a tag contains a character other
  than `a-z`, `0-9` and `-`
- **THEN** the dialog cannot proceed, and a bad tag is named

#### Scenario: A prompt's description is kept

- **WHEN** a prompt is submitted with a description
- **THEN** the prompt keeps it, and the next submission starts from it

### Requirement: A submission is reviewed before it is published

Proceeding SHALL show the file as it will be proposed, editable, and SHALL
NOT open anything until the user confirms it. The file SHALL carry the
item's own id. What is published SHALL be the reviewed text. Text that looks
like a home folder, an email address or a credential SHALL be flagged.

#### Scenario: Nothing is published without confirmation

- **WHEN** the user moves from the details to the review step
- **THEN** the file is shown for editing and nothing has opened

#### Scenario: Local values are flagged

- **WHEN** the reviewed file holds the user's home folder path
- **THEN** the review step warns before it can be opened on GitHub

### Requirement: A submission opens on GitHub as a new item or an update

Confirming SHALL open Knot-Library on GitHub in the browser; Knot SHALL NOT
write to the library itself. An item whose id the library's index already
has SHALL be an update to that file; any other SHALL be a new file at
`<kind-dir>/<slug>.md`. A file the page cannot carry SHALL be put on the
clipboard, and the user SHALL be told which happened.

#### Scenario: A new prompt opens as a new library file

- **WHEN** the user confirms the prompt "Code review", whose id the library
  does not have
- **THEN** Knot-Library's new-file page opens at `prompts/code-review.md`
  carrying the reviewed file

#### Scenario: An item the library has is an update

- **WHEN** the user confirms a prompt imported from Knot-Library and edited
- **THEN** the library's existing file for that id opens for editing, and the
  reviewed file is on the clipboard to replace it

#### Scenario: A long file goes to the clipboard

- **WHEN** a new file is too long to carry in the page's address
- **THEN** the file is on the clipboard and the new-file page opens empty

### Requirement: Duplicating a prompt from the Prompts window

Each row of the Prompts window SHALL carry a Duplicate action beside its edit
and delete actions. It SHALL duplicate the prompt (see `prompt-library` -
Library operations), show the copy in the list right after the original, and
open the prompt editor on the copy with the caret in its name.

The copy SHALL be in the library before the editor opens. The editor SHALL
treat it as an existing prompt: its confirm action SHALL be "Save", and
canceling SHALL leave the copy as it was made.

#### Scenario: Duplicating a prompt

- **WHEN** the user clicks Duplicate on "Review"
- **THEN** "Review copy" appears right after "Review", and the prompt editor
  opens on it with the caret in its name

#### Scenario: Canceling the copy's editor keeps the copy

- **WHEN** the user duplicates "Review" and cancels the editor that opens
- **THEN** "Review copy" stays in the library, unchanged

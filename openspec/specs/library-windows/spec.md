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
confirmation dialog.

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

- **WHEN** a persona is assigned to two agents
- **THEN** its delete action is disabled, and its tooltip says it is in use
  by 2 agents

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
text field's caret. It SHALL flag unknown variable names as warnings without
disabling save (see `prompt-library` - Unknown variables and escaping).

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

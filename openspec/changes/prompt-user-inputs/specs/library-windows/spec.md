## MODIFIED Requirements

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

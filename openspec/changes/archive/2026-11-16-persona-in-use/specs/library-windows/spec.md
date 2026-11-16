# Spec Delta

## MODIFIED Requirements

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

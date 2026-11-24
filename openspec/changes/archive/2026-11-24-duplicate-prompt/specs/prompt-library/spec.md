## MODIFIED Requirements

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

## ADDED Requirements

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

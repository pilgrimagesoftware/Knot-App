## ADDED Requirements

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

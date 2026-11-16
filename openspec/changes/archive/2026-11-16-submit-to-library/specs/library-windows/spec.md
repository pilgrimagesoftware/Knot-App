## ADDED Requirements

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

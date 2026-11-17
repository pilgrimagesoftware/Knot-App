# acp-panel-ui delta

## ADDED Requirements

### Requirement: Question cards
The system SHALL render a pending form elicitation inline in the conversation,
where a permission prompt goes, as a question card: each field's title and
description, a single-choice field as a list to pick one from, a
multiple-choice field as a list to check any number in, each choice's
description beside it, and text, number and boolean fields as matching
inputs. A text field `<key>_custom` right after the choice field `<key>` SHALL
be shown under it as that question's "Other" answer.

#### Scenario: A single- and a multiple-choice question
- **WHEN** Claude asks "Which color?" (Red, Green, Blue) and "Which fruits?"
  (Apple, Pear, Plum, multiple)
- **THEN** the card shows both questions with their headers, one pickable
  list and one checkable list, each with its "Other" field below it

#### Scenario: Answering with Other
- **WHEN** the user picks no color, types "Teal" in that question's Other
  field, and submits
- **THEN** "Teal" is sent as that question's free-text answer

### Requirement: Submitting or skipping a question card
A question card SHALL offer **Submit**, which sends the answers, and **Skip**,
which declines. Submit SHALL be unavailable while a required field is empty or
a number field does not hold a number. The card SHALL be removed once the
request is answered, cancelled or withdrawn, and text in the composer SHALL be
kept.

#### Scenario: Submitting answers
- **WHEN** the user picks Red, checks Apple and Plum, and presses Submit
- **THEN** those answers are sent to the agent and the card is removed

#### Scenario: Skipping
- **WHEN** the user presses Skip
- **THEN** the form is declined and the card is removed

#### Scenario: A required field blocks Submit
- **WHEN** an MCP server's form marks a field required and it is empty
- **THEN** Submit is unavailable until the field is filled

#### Scenario: Stop removes the card
- **WHEN** the user presses Stop while a question card is shown
- **THEN** the form is cancelled and the card is removed

### Requirement: Question card takes focus
When a question card appears on the visible panel it SHALL take keyboard
focus, and focus SHALL return to the composer when the card is removed. The
permission allow and deny keybindings SHALL NOT answer a question card.

#### Scenario: Permission shortcuts do not answer a question
- **WHEN** a question card is shown and the user invokes the permission allow
  or deny keybinding
- **THEN** no answer is sent and the card stays

### Requirement: Question card keys
With a question card focused: Up and Down SHALL move between choices and on
into the next field; Tab and Shift-Tab between fields; Space SHALL pick a
single choice or toggle a multiple one; Return SHALL pick the focused single
choice and move to the next question's first choice, past the Other field;
Cmd-Return SHALL submit; Escape SHALL skip. Submit and Skip SHALL show their
bound keystroke, read from the installed keymap.

#### Scenario: Answering from the keyboard
- **WHEN** a card with a single-choice and a multiple-choice question appears
  and the user presses Down, Return, Space, then Cmd-Return
- **THEN** the second choice of the first question is picked, the first
  choice of the second question is checked, and the answers are sent

#### Scenario: Escape skips
- **WHEN** the card has focus and the user presses Escape
- **THEN** the form is declined

### Requirement: Keys inside a question card's text field
With focus in a question card's text or number field, typed keys SHALL edit
the field; Return and Down SHALL move to the next field, Up and Shift-Tab to
the previous one; Cmd-Return SHALL submit. Escape SHALL leave the field for
the card without skipping, and a second Escape SHALL skip.

#### Scenario: Return in an Other field moves on
- **WHEN** the user types "Teal" in the first question's Other field and
  presses Return
- **THEN** "Teal" stays in the field and focus moves to the next question

#### Scenario: One Escape does not throw away typed text
- **WHEN** the user has typed into an Other field and presses Escape once
- **THEN** focus leaves the field, the text is kept, and the form is not
  declined

### Requirement: A pending question gates the session like a pending permission
While a question card is pending, the system SHALL treat the session exactly
as it treats a pending permission request: a prompt the user sends is queued
rather than sent, queued prompts wait, steering waits, and the agent's status
is Awaiting input with the first question (or the form's message) as its
detail, raising the awaiting-input notification once. Pending questions and
permission requests SHALL be shown one at a time, in the order they arrived.

#### Scenario: Sending while a question is pending queues
- **WHEN** a question card is pending and the user sends a prompt
- **THEN** the prompt is queued and delivered after the question is answered
  and the turn ends

#### Scenario: The agent shows Awaiting input
- **WHEN** Claude asks "Which color?" through a question card
- **THEN** the agent's status is Awaiting input with "Which color?" as the
  detail, and one desktop notification is raised if the agent is not visible

#### Scenario: A permission request arriving behind a question waits
- **WHEN** a question card is shown and a permission request arrives
- **THEN** the permission prompt is shown after the question is answered,
  not over it

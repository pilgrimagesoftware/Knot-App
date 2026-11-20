## MODIFIED Requirements

### Requirement: Submitting or skipping a question card
A question card SHALL offer **Submit**, which sends the answers, and **Skip**,
which declines. Submit SHALL be unavailable while a required field is empty,
a number field does not hold a number, or a choice question with an "Other"
field has neither a pick nor non-blank Other text: the agent cannot tell an
empty submission from no answer, and Skip is how a question goes unanswered. The card SHALL be removed once the
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

#### Scenario: An unanswered question blocks Submit
- **WHEN** Claude asks one question with three options and the user has
  picked none and typed nothing in Other
- **THEN** Submit is unavailable until an option is picked or Other holds text

#### Scenario: Stop removes the card
- **WHEN** the user presses Stop while a question card is shown
- **THEN** the form is cancelled and the card is removed

### Requirement: Question card keys
Focus SHALL start on the first choice of the first field. The focused choice
SHALL be marked with an outline, not a fill, so it does not look picked. Up and Down SHALL
move between choices and on into the next field; Tab and Shift-Tab between
fields; Space SHALL pick a single choice or toggle a multiple one; Return
SHALL pick the focused single choice and move to the next question's first
choice, past the Other field; Cmd-Return SHALL submit; Escape SHALL skip.
Submit and Skip SHALL show their bound keystroke, read from the installed
keymap.

#### Scenario: Answering from the keyboard
- **WHEN** a card with a single-choice and a multiple-choice question appears
  and the user presses Down, Return, Space, then Cmd-Return
- **THEN** the second choice of the first question is picked, the first
  choice of the second question is checked, and the answers are sent

#### Scenario: Escape skips
- **WHEN** the card has focus and the user presses Escape
- **THEN** the form is declined

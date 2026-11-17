# acp-client delta

## MODIFIED Requirements

### Requirement: Capability negotiation
On connecting to a subprocess, the system SHALL send an `initialize` request
carrying its own protocol version and capabilities, and SHALL record the
agent's declared protocol version and capabilities from the response before
issuing any other request.

The client capabilities SHALL advertise form-mode elicitation
(`clientCapabilities.elicitation.form`). They SHALL NOT advertise URL-mode
elicitation.

#### Scenario: Unsupported protocol version
- **WHEN** the agent's `initialize` response reports a protocol version the
  client does not support
- **THEN** the system SHALL fail the connection with a typed error instead of
  proceeding to session creation

#### Scenario: Form elicitation is advertised
- **WHEN** the system sends `initialize` to any agent
- **THEN** the request's client capabilities include `elicitation.form`, and
  do not include `elicitation.url`

## ADDED Requirements

### Requirement: Form elicitation requests
When the agent sends an `elicitation/create` request in form mode, the system
SHALL parse its schema into typed fields (single choice, multiple choice,
text, number, boolean) in the order the agent listed them, SHALL surface it to
the caller as a distinct event carrying the fields, the form's message and the
tool call's id when there is one, and SHALL hold the reply until the caller
answers.

#### Scenario: A question with choices is surfaced in order
- **WHEN** the agent sends a form with a single-choice field, its free-text
  field, a multiple-choice field and its free-text field, in that order
- **THEN** the surfaced event lists the four fields in that order, each
  choice with its value, title and description

#### Scenario: Field order survives more than nine fields
- **WHEN** the agent sends a form whose fields are named `question_0` to
  `question_11`
- **THEN** the fields are surfaced in the order sent, not sorted by name

#### Scenario: Stop cancels a pending form
- **WHEN** the caller cancels the session's turn while a form is pending
- **THEN** the system replies `cancel` to the form

#### Scenario: Session end cancels a pending form
- **WHEN** the session ends while a form is pending
- **THEN** the system replies `cancel` rather than leaving it unanswered

### Requirement: Form answers are typed per field
The caller's answer to a form SHALL be sent as `accept` with each filled field
typed as the field declares (a string for a single choice or text, an array of
strings for a multiple choice, a number, a boolean) and empty optional fields
left out, as `decline` when the user skips, or as `cancel`.

#### Scenario: Answers are typed per field
- **WHEN** the caller submits a single choice, two choices of a multiple-choice
  field and leaves an optional text field empty
- **THEN** the reply is `accept` with the single choice as a string, the
  multiple choice as an array of two strings, and no entry for the empty field

#### Scenario: Skip declines
- **WHEN** the caller skips the form
- **THEN** the reply is `decline`

### Requirement: Unrenderable forms are refused
An `elicitation/create` the system cannot render (URL mode, a nested object, a
field type outside those the system parses) SHALL be answered at once with a
JSON-RPC invalid-params error and SHALL NOT be surfaced to the caller.

#### Scenario: An unrenderable form is refused without a card
- **WHEN** the agent sends an `elicitation/create` in URL mode, or a form with
  a nested object field
- **THEN** the system answers with an invalid-params error at once and
  surfaces no event

### Requirement: Withdrawn requests
When the agent sends `$/cancel_request` for a request it is still waiting on
(a permission request or a form elicitation), the system SHALL answer that
request with the request-cancelled error (`-32800`), SHALL stop waiting for
the caller's answer, and SHALL tell the caller the request is resolved so its
prompt is removed. A `$/cancel_request` for an id that is not pending SHALL be
ignored.

#### Scenario: The agent withdraws a question
- **WHEN** a form elicitation is pending and the agent sends
  `$/cancel_request` with its id
- **THEN** the system answers it with `-32800` and the caller is told it is
  resolved

#### Scenario: The agent withdraws a permission request
- **WHEN** a permission request is pending and the agent sends
  `$/cancel_request` with its id
- **THEN** the system answers it with `-32800` and the caller is told it is
  resolved, so the permission prompt is removed

#### Scenario: A late answer is not sent
- **WHEN** the caller answers a request after the agent withdrew it
- **THEN** no second reply is sent for that id

#### Scenario: An unknown id is ignored
- **WHEN** the agent sends `$/cancel_request` for an id that is not pending
- **THEN** nothing is sent and no prompt changes

# Spec Delta

## MODIFIED Requirements

### Requirement: Tri-classification

Given a non-empty last assistant message, the system SHALL classify it
into exactly one of `completed`, `binary`, or `open` by calling the
configured provider with a fixed system prompt and parsing the response:
a reply equal to or prefixed by `binary` (case-insensitive) is `binary`,
one equal to or prefixed by `open` is `open`, and anything else - including
a request failure - is `completed`.

The system prompt SHALL classify a message as `binary` or `open` only when it
asks the user a question or for a decision the agent needs before it can go
on, and SHALL name as `completed` a status report or summary, a message saying
nothing is needed from the user, a request addressed to another agent, and a
closing courtesy after finished work.

#### Scenario: Unrecognized response defaults to completed

- **WHEN** the provider's response is neither `binary` nor `open`
- **THEN** the classification is `completed`

#### Scenario: Request failure yields no action

- **WHEN** the classification request fails (missing key, unsupported
  provider, network error)
- **THEN** the classification is treated as `completed` and no action is
  dispatched

#### Scenario: A report that needs nothing is completed

- **WHEN** the system prompt is read
- **THEN** it lists a status report stating that nothing needs the user's
  input among its `completed` examples

### Requirement: One classify-and-land path

Both triggers SHALL go through one path that checks enablement, the API key
and a non-blank message, classifies off the UI thread, and lands the outcome
identically whichever trigger fired. Landing `mark` or `ask` SHALL set the
agent to Awaiting input and raise the awaiting-input notification; landing a
reply SHALL do neither, since the reply starts the agent's next turn.

Landing `mark` or `ask` SHALL record why the agent waits - that Autopilot
stopped it, and whether the reply asked for a go-ahead or an open question, or
that the custom action could not check it - as the agent's awaiting reason, and
SHALL carry the same text as the notification's message.

#### Scenario: A mark waits on the user

- **WHEN** a classification lands as `mark`
- **THEN** the agent is Awaiting input and the awaiting-input notification is
  raised

#### Scenario: A mark says why

- **WHEN** a `binary` classification lands as `mark`
- **THEN** the agent's awaiting reason, and the notification's message, say
  that Autopilot read the last reply as asking for a go-ahead

#### Scenario: An injected reply does not mark the agent

- **WHEN** a classification lands as a reply to inject
- **THEN** the agent's status is left as it was and no notification is raised

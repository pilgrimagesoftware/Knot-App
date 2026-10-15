# autopilot Specification

## Purpose

Defines when a completed turn is classified and what happens next: the two
triggers (a Terminal-mode agent's hook, per `agent-hooks`'s "Autopilot
classification trigger", and a Panel-mode agent's turn ending in its
workspace window), the tri-classification prompt and parsing, the action
matrix (mark/ask/auto-continue/custom), the LLM providers called, the effect
each outcome produces, how a reply reaches the agent, and what the
conversation shows of it.

## Requirements

### Requirement: Tri-classification

Given a non-empty last assistant message, the system SHALL classify it
into exactly one of `completed`, `binary`, or `open` by calling the
configured provider with a fixed system prompt and parsing the response:
a reply equal to or prefixed by `binary` (case-insensitive) is `binary`,
one equal to or prefixed by `open` is `open`, and anything else - including
a request failure - is `completed`.

#### Scenario: Unrecognized response defaults to completed

- **WHEN** the provider's response is neither `binary` nor `open`
- **THEN** the classification is `completed`

#### Scenario: Request failure yields no action

- **WHEN** the classification request fails (missing key, unsupported
  provider, network error)
- **THEN** the classification is treated as `completed` and no action is
  dispatched

### Requirement: Action matrix

Given a classification other than `completed`, the system SHALL dispatch
per the configured action: `mark` always marks the agent as needing input
and notifies; `ask` opens a decision dialog carrying the message and
classification; `continue` auto-injects `"yes, continue"` for a `binary`
classification and falls back to marking for `open` (an open question
cannot be auto-answered); `custom` is never reached here - see the
custom-action requirement below. A `completed` classification SHALL
dispatch nothing regardless of the configured action.

#### Scenario: Continue falls back to mark for an open question

- **WHEN** the configured action is `continue` and the classification is
  `open`
- **THEN** the agent is marked as needing input rather than auto-answered

### Requirement: Custom action

When the configured action is `custom`, the system SHALL skip
classification and instead send the agent's last message to the
configured provider with a fixed framing prompt followed by the user's own
custom prompt. A response that is empty, whitespace-only, or the literal
`EMPTY` (case-insensitive) SHALL be treated as no action; any other
response SHALL be injected into the agent as if typed by the user,
trimmed of surrounding whitespace. A request failure SHALL fall back to
marking the agent as needing input, unlike a plain classification failure.
An empty custom prompt SHALL skip the request entirely and take no action.

#### Scenario: Empty custom prompt takes no action without calling the provider

- **WHEN** the action is `custom` and no custom prompt is configured
- **THEN** no request is made and no action is dispatched

#### Scenario: Custom request failure marks rather than staying silent

- **WHEN** the action is `custom` and the request fails
- **THEN** the agent is marked as needing input

### Requirement: Supported providers

The system SHALL support OpenAI, Anthropic, and Google as classification
and custom-action providers, each called with a fixed, cheap model for the
provider and the configured API key. A missing or whitespace-only API key
SHALL fail the request without making any call.

#### Scenario: Blank API key fails fast

- **WHEN** the configured API key is empty or whitespace-only
- **THEN** no network request is made and the call fails

### Requirement: Panel-mode turn-end trigger

A Panel-mode agent posts no hooks, so its workspace window SHALL trigger
classification itself when the agent's reported status moves from Working to
Idle, classifying the turn's reply: the last assistant message after the
latest prompt. A turn with no such reply SHALL NOT be classified. The hook
trigger's enablement and API-key conditions SHALL apply.

#### Scenario: A finished panel turn is classified

- **WHEN** autopilot is enabled with an API key, and a Panel-mode agent's turn
  ends with assistant text
- **THEN** that text is classified, without blocking the window

#### Scenario: A permission request is not a finished turn

- **WHEN** a Panel-mode agent's status moves from Working to Awaiting input
- **THEN** no classification is started

#### Scenario: A turn that ended in an error is not classified

- **WHEN** a Panel-mode agent's turn ends with an error after partial text
- **THEN** no classification is started

#### Scenario: A tool-only turn is not classified

- **WHEN** a Panel-mode agent's turn ends having run tools but written no text
- **THEN** no classification is started

#### Scenario: A session's first status is not a turn ending

- **WHEN** a Panel-mode agent's session first reports Idle
- **THEN** no classification is started

### Requirement: One classify-and-land path

Both triggers SHALL go through one path that checks enablement, the API key
and a non-blank message, classifies off the UI thread, and lands the outcome
identically whichever trigger fired. Landing `mark` or `ask` SHALL set the
agent to Awaiting input and raise the awaiting-input notification; landing a
reply SHALL do neither, since the reply starts the agent's next turn.

#### Scenario: A mark waits on the user

- **WHEN** a classification lands as `mark`
- **THEN** the agent is Awaiting input and the awaiting-input notification is
  raised

#### Scenario: An injected reply does not mark the agent

- **WHEN** a classification lands as a reply to inject
- **THEN** the agent's status is left as it was and no notification is raised

### Requirement: Reply delivery

A reply Autopilot sends - `continue`'s `"yes, continue"`, a `custom` response,
or the decision dialog's "Yes, Continue" - SHALL reach the agent as a submitted
prompt. A Panel-mode agent SHALL receive it as a prompt of origin `Autopilot`:
queued behind a running turn like any other, following the response only for a
reader already at the conversation's tail, and labelled as an Autopilot reply
while it waits in the queue. A Terminal-mode agent SHALL receive it typed and
submitted with Return.

#### Scenario: A reply waits behind a running turn and says whose it is

- **WHEN** Autopilot sends a reply to a Panel-mode agent whose turn is still
  running
- **THEN** the reply is queued, and its queue row reads as an Autopilot reply

#### Scenario: A terminal reply is submitted

- **WHEN** Autopilot sends a reply to a Terminal-mode agent
- **THEN** the text is written to the terminal followed by Return

### Requirement: Conversation notices

Each time Autopilot acts on a Panel-mode agent, the agent's conversation SHALL
gain a notice line saying so, distinct from the agent's own messages: one for
`mark` (including `continue` falling back to marking), one for `ask` as its
dialog opens, and one for every reply, placed directly before the reply's
prompt. The notices SHALL NOT start a turn and SHALL come from the localization
catalog. They are local to the open window and are not part of the agent's
session history.

#### Scenario: A mark is visible in the conversation

- **WHEN** a classification lands as `mark` for a Panel-mode agent
- **THEN** the conversation ends with Autopilot's "needs your input" notice

#### Scenario: A reply is announced

- **WHEN** Autopilot sends a reply to a Panel-mode agent
- **THEN** the "replied for you" notice precedes the reply in the conversation

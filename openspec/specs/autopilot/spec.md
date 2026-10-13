# autopilot Specification

## Purpose

Defines what happens once `agent-hooks`'s "Autopilot classification
trigger" requirement decides a completed turn should be classified: the
tri-classification prompt and parsing, the action matrix (mark/ask/
auto-continue/custom), the LLM providers called, and the effect each
outcome produces.

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

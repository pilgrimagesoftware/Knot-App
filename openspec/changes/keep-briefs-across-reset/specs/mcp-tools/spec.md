# Spec Delta

## MODIFIED Requirements

### Requirement: send-message, check-messages, broadcast-message

`send-message` SHALL require `from`, `to`, `content`. `check-messages` SHALL
require `agentId` and accept optional `markAsRead` (default true),
`includeRead` (default false) and `since`, an RFC 3339 time in the form each
message's `timestamp` is reported in; a `since` that is not one SHALL be
rejected with `isError` true. Each listed message SHALL carry `read`.
`broadcast-message` SHALL require `from`, `content`. Their delivery, routing,
and rejection semantics are defined by `mcp-messaging`; on rejection the tool
result SHALL carry the rejection string with `isError` true.

#### Scenario: Rejected send surfaces the reason

- **WHEN** `send-message` targets a shell agent
- **THEN** the tool result text is "Cannot send messages to shell agents" with
  `isError` true

#### Scenario: A read brief can be checked again

- **WHEN** an agent that has already read a brief calls `check-messages` with
  `includeRead` true
- **THEN** the brief is listed with `read` true

#### Scenario: A since that is not a time

- **WHEN** `check-messages` is called with `since` set to `yesterday`
- **THEN** the tool result has `isError` true

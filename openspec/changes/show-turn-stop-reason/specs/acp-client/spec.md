# Spec Delta

## ADDED Requirements

### Requirement: A turn's stop reason is kept

The system SHALL take a turn's stop reason from the response to
`session/prompt`, defaulting to `end_turn` when the response names none, and
SHALL deliver it with the turn-end event as one of the spec's stop reasons, as
`error` when the prompt request failed or its connection closed, or verbatim
when the agent sent a value the spec does not name. The system SHALL write the
stop reason to its log.

#### Scenario: An early stop reaches the caller

- **WHEN** the agent answers `session/prompt` with stop reason `max_tokens`
- **THEN** the turn-end event carries `max_tokens`, and the log records it

#### Scenario: A failed prompt still ends the turn

- **WHEN** the `session/prompt` request fails
- **THEN** a turn-end event carrying `error` is delivered

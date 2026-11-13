# Spec Delta

## ADDED Requirements

### Requirement: A turn the agent stopped early says why

When a turn ends with a stop reason other than `end_turn`, `cancelled` or
`error`, the panel SHALL add a notice to the conversation naming the reason:
the output token limit, the turn's limit of model requests, a refusal, or the
reason as the agent sent it. A turn that ended with `end_turn`, `cancelled` or
`error` SHALL add no notice.

#### Scenario: Out of output tokens

- **WHEN** an agent's turn ends with stop reason `max_tokens`
- **THEN** the conversation ends with a notice that the turn stopped at the
  output token limit, and the agent is idle

#### Scenario: A finished turn says nothing more

- **WHEN** an agent's turn ends with stop reason `end_turn`
- **THEN** no notice is added

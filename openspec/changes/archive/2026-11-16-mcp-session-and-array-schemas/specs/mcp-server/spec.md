# Spec Delta

## ADDED Requirements

### Requirement: A session id comes from initialize

The server SHALL return an `Mcp-Session-Id` only in answer to `initialize`,
or to a request that sent one: a known id is returned unchanged, and an
unknown id is replaced by a new session. A request that is not `initialize`
and sends no id SHALL be answered without one.

#### Scenario: A probe before initialize

- **WHEN** a client sends `server/discover` with no session id, then
  `initialize`, then `tools/list` with the id `initialize` returned
- **THEN** the first answer carries no session id, and `tools/list` is
  answered with the same id `initialize` gave

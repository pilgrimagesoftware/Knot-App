## MODIFIED Requirements

### Requirement: Hook routes and request shape

The system SHALL accept `POST /api/v1/agent/register` and
`POST /api/v1/agent/status`, each requiring a credential per `mcp-server`
"Client authentication". Every hook request body SHALL be JSON containing at
least `agent_id` (a uuid string); a missing or unparseable `agent_id` SHALL
yield a 400. With an agent credential, `agent_id` SHALL equal that
credential's agent, or the request SHALL yield a 403 and change nothing. With
the client token, `agent_id` SHALL NOT name an agent whose credential-bound
MCP connection is live, or the request SHALL yield a 403. The body MAY contain
`agent` (the agent type, default `claude`), `session_id`, `source`, `hook`,
`status`, and a nested `payload` object. Hook scripts shipped with Knot SHALL
read the server URL and credential from `KNOT_MCP_URL` and `KNOT_MCP_TOKEN`.

#### Scenario: Invalid agent id

- **WHEN** a hook request with a valid credential omits `agent_id`
- **THEN** the response is 400 and no state changes

#### Scenario: Unknown agent type

- **WHEN** a hook request with a valid credential carries `agent` set to a
  type with no handler
- **THEN** the response is 400 naming the unknown type

#### Scenario: A hook for another agent

- **WHEN** a hook request presents agent A's credential with `agent_id` set to agent B
- **THEN** the response is 403 and B's status and metadata are unchanged

#### Scenario: A hook with no credential

- **WHEN** a hook request carries no credential
- **THEN** the response is 401 and no state changes

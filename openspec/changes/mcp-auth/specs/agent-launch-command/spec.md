## ADDED Requirements

### Requirement: Agent credential delivery

When it launches an ACP agent with the MCP server enabled, the system SHALL
issue that agent a credential (see `mcp-server` "Credential kinds") and pass
it as `Authorization: Bearer <credential>` in the ACP MCP server entry's
`headers`, and as `KNOT_MCP_TOKEN` (with `KNOT_MCP_URL`) in the adapter
subprocess environment for hook scripts. The credential SHALL NOT appear in
the URL, the adapter's command line, or any log line.

#### Scenario: The credential travels in a header

- **WHEN** an ACP agent is launched with the MCP server enabled
- **THEN** the `mcpServers` entry has an `Authorization` header and its URL
  contains no credential

## MODIFIED Requirements

### Requirement: MCP configuration for ACP-launched agents

The system SHALL configure MCP access for every ACP-launched (non-shell)
agent through the `session/new`/`session/load` request's MCP server
parameters, naming the knot HTTP MCP server and its URL, rather than through
a CLI argument passed to the adapter subprocess, uniformly across every agent
type with a registered ACP adapter. The entry's `headers` SHALL carry that
agent's credential (see "Agent credential delivery").

#### Scenario: ACP session carries MCP server config

- **WHEN** a non-shell agent with a registered ACP adapter is launched and
  the MCP server is enabled
- **THEN** its `session/new` (or `session/load`) request includes the knot
  MCP server's URL and an `Authorization` header with that agent's
  credential, and the agent can successfully call knot tools (e.g.
  `set-status`, `register-agent`) once the session starts

#### Scenario: A resumed session gets the current credential

- **WHEN** Knot restarts and resumes an agent with `session/load`
- **THEN** the request carries a credential issued in this run, and the
  previous run's credential is not accepted

#### Scenario: MCP disabled omits ACP MCP config

- **WHEN** the MCP server is disabled
- **THEN** the ACP session request includes no MCP server configuration, and
  no credential is issued

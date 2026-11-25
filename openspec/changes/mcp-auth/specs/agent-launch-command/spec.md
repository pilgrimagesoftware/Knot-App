## MODIFIED Requirements

### Requirement: MCP configuration for ACP-launched agents

The system SHALL configure MCP access for every ACP-launched (non-shell)
agent through the ACP protocol's own session-scoped mechanism (the
`session/new`/`session/load` request's MCP server parameters), naming the
knot HTTP MCP server and its URL, rather than through a CLI argument string
passed to the adapter subprocess. This SHALL apply uniformly across every
agent type with a registered ACP adapter, replacing the previous
per-type CLI-flag behavior, which never reached the subprocess correctly.
The entry's `headers` SHALL carry `Authorization: Bearer <agent credential>`
for that agent (see `mcp-server` "Client authentication"), issued when the
agent is launched; the credential SHALL NOT appear in the URL, in the
adapter's command line, or in any log line. The adapter subprocess
environment SHALL carry `KNOT_MCP_URL` and `KNOT_MCP_TOKEN` (the same agent
credential) for hook scripts.

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

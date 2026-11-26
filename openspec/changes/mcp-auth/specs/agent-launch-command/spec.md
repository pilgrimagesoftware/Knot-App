## ADDED Requirements

### Requirement: Agent credential delivery

When it launches an ACP agent with the MCP server enabled, the system SHALL
issue that agent a tool and a hook credential (see `mcp-server` "Credential
kinds"). The tool credential SHALL be passed only as `Authorization: Bearer`
in the ACP MCP server entry's `headers`; the hook credential only as
`KNOT_MCP_TOKEN` (with `KNOT_MCP_URL`) in the adapter environment. Neither
SHALL appear in the URL, the adapter command line Knot builds, or any log.

#### Scenario: The credential travels in a header

- **WHEN** an ACP agent is launched with the MCP server enabled
- **THEN** the `mcpServers` entry has an `Authorization` header with the tool
  credential, its URL contains no credential, and the adapter environment's
  `KNOT_MCP_TOKEN` holds the hook credential, not the tool credential

### Requirement: Restart rotates an agent's credentials atomically

Restarting an agent (with or without a new conversation) SHALL issue it new
tool and hook credentials and revoke the previous pair in one step, before
the new adapter starts, so that no moment exists in which both pairs are
accepted. Closing an agent SHALL revoke its pair.

#### Scenario: The old credential is refused after a restart

- **WHEN** an agent is restarted and its previous process, still shutting
  down, sends a request with the previous tool credential
- **THEN** the response is 401, and the new session's credential is accepted

#### Scenario: Restart with a new conversation

- **WHEN** an agent is restarted with a new conversation
- **THEN** the previous tool and hook credentials are both refused

### Requirement: Adapter-side credential exposure is surfaced

When Knot launches the Claude adapter in a configuration that makes it pass
MCP servers on the Claude CLI's command line (`CLAUDE_CODE_EXECUTABLE` in the
environment Knot passes, or a user option named `settings` or `sdk-url`), the
agent's panel SHALL show a notice saying so, and Knot SHALL NOT change the
user's options to avoid it.

#### Scenario: A user option selects argv delivery

- **WHEN** a Claude agent is launched with a user option named `settings`
- **THEN** its panel shows the command-line delivery notice and the agent
  starts with the user's options unchanged

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

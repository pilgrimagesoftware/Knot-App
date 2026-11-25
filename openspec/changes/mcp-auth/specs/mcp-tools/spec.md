## MODIFIED Requirements

### Requirement: A caller acts only as itself

Every tool names its caller in an argument: `from` for `send-message` and
`broadcast-message`, `agentId` for every other tool. The server SHALL bind an
MCP connection to an agent only from an agent credential (see `mcp-server`
"Client authentication"): a connection presenting agent `x`'s credential is
bound to `x`. The URL's `agent` query parameter (`…/mcp?agent=<id>`), which
Knot SHALL keep putting in the URL it gives each agent it launches, is a hint
only: when present it SHALL name the credential's agent, or the request SHALL
be refused with 403. A new connection bound by `x`'s credential SHALL replace
`x`'s previous connection only when that connection was also bound by `x`'s
credential; nothing else SHALL displace it.

A call whose caller argument does not resolve to the connection's bound agent
SHALL fail with an error that names the bound agent and its id, and SHALL
change nothing. A connection authenticated by the client token is bound to no
agent; on it, a call whose caller argument names an agent that a live
connection holds SHALL fail the same way. A call naming no caller, or naming
an agent Knot does not know on a connection bound to none, SHALL be left to
the tool.

#### Scenario: A connection naming another agent is refused

- **WHEN** a connection authenticated with Builder 1's credential calls
  `set-status` with the orchestrator's id
- **THEN** the call fails, the orchestrator's status is unchanged, and the
  error gives Builder 1's id

#### Scenario: A URL hint that disagrees with the credential

- **WHEN** a request presents Builder 1's credential on `…/mcp?agent=<orchestrator id>`
- **THEN** the response is 403 and no session is bound

#### Scenario: A restarted agent takes over its connection

- **WHEN** an agent is restarted and its new session connects with its credential
- **THEN** the new connection holds the agent, and its registration succeeds

#### Scenario: The client token cannot displace an agent

- **WHEN** a connection authenticated by the client token calls
  `register-agent` for an agent whose credential-bound connection is live
- **THEN** the call fails, and the agent's live connection is unchanged

### Requirement: A bound connection registers its agent

A request on an MCP connection bound to an agent by its agent credential
SHALL mark that agent registered, when Knot knows it and it is not registered
already. That happens with no tool call and no model turn, and on every such
request, so an agent the registry did not yet hold when it connected is
registered as soon as the registry does. Nothing SHALL be persisted, since
registration is runtime state. `register-agent` SHALL go on registering an
agent from a connection authenticated by the client token, and SHALL keep
refusing an agent another live connection holds.

#### Scenario: A resumed agent is registered by its connection

- **WHEN** Knot restarts and an agent's resumed session connects with its credential
- **THEN** `list-agents` shows it registered before it makes any tool call

#### Scenario: An unbound connection registers nobody by connecting

- **WHEN** a connection authenticated by the client token initializes
- **THEN** no agent's registration changes

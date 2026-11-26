## ADDED Requirements

### Requirement: Identity comes from the agent credential

The server SHALL bind an MCP connection to agent `x` only when it presents
`x`'s agent credential (see `mcp-server` "Client authentication"); a
connection authenticated by the client token is bound to no agent. The URL's
`agent` query parameter, which Knot keeps in each launched agent's URL, is a
hint: when present it SHALL name the credential's agent, or the request SHALL
get 403. A connection bound by `x`'s credential SHALL be replaced only by
another connection presenting `x`'s credential.

#### Scenario: A URL hint that disagrees with the credential

- **WHEN** a request presents Builder 1's credential on `…/mcp?agent=<orchestrator id>`
- **THEN** the response is 403 and no session is bound

#### Scenario: The client token cannot displace an agent

- **WHEN** a connection authenticated by the client token calls
  `register-agent` for an agent whose credential-bound connection is live
- **THEN** the call fails, and the agent's live connection is unchanged

### Requirement: MCP-created commands and distant worktrees need confirmation

An MCP call to `create-agent` with `agentType: shell` and a non-empty
`shellCommand`, or one that creates a worktree (via `create-worktree` or
`createWorktree: true`) whose canonical destination is outside the
repository's parent directory, SHALL wait for the user to confirm a dialog
naming the requesting agent, the command and the folder. A denial or timeout
SHALL return a tool error and create nothing.

#### Scenario: The user confirms a shell command

- **WHEN** an agent calls `create-agent` with `agentType: shell` and
  `shellCommand: "make test"`, and the user confirms
- **THEN** the agent is created and its command runs

#### Scenario: The user declines

- **WHEN** the user declines the confirmation
- **THEN** the call fails with a tool error and no agent, worktree or folder
  is created

#### Scenario: A plain shell needs no confirmation

- **WHEN** an agent calls `create-agent` with `agentType: shell` and no
  `shellCommand`
- **THEN** the agent is created without a confirmation

#### Scenario: A worktree beside the repository needs no confirmation

- **WHEN** an agent calls `create-worktree` with no `worktreePath`
- **THEN** the worktree is created at the suggested sibling path without a
  confirmation

#### Scenario: A distant worktree needs confirmation

- **WHEN** an agent calls `create-worktree` with a `worktreePath` outside the
  repository's parent directory
- **THEN** the call waits for the user's confirmation

## MODIFIED Requirements

### Requirement: A caller acts only as itself

Every tool names its caller in an argument: `from` for `send-message` and
`broadcast-message`, `agentId` for every other tool. A call whose caller
argument does not resolve to the connection's bound agent (see "Identity
comes from the agent credential") SHALL fail with an error naming the bound
agent and its id, and SHALL change nothing. On an unbound connection, a call
naming an agent that a live connection holds SHALL fail the same way. A call
naming no caller, or an unknown agent on an unbound connection, SHALL be left
to the tool.

#### Scenario: A connection naming another agent is refused

- **WHEN** a connection authenticated with Builder 1's credential calls
  `set-status` with the orchestrator's id
- **THEN** the call fails, the orchestrator's status is unchanged, and the
  error gives Builder 1's id

#### Scenario: A restarted agent takes over its connection

- **WHEN** an agent is restarted and its new session connects with its credential
- **THEN** the new connection holds the agent, and its registration succeeds

### Requirement: A bound connection registers its agent

A request on a connection bound to an agent by its credential SHALL mark that
agent registered when Knot knows it and it is not registered already, with no
tool call or model turn, on every such request. Nothing SHALL be persisted.
`register-agent` SHALL go on registering an agent from a connection
authenticated by the client token, and SHALL keep refusing an agent another
live connection holds.

#### Scenario: A resumed agent is registered by its connection

- **WHEN** Knot restarts and an agent's resumed session connects with its credential
- **THEN** `list-agents` shows it registered before it makes any tool call

#### Scenario: An unbound connection registers nobody by connecting

- **WHEN** a connection authenticated by the client token initializes
- **THEN** no agent's registration changes

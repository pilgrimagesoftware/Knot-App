## MODIFIED Requirements

### Requirement: MCP tab

The MCP tab SHALL show an "Enable MCP server" toggle bound to
`mcp_server_enabled`, a port field bound to `mcp_server_port`, a read-only
server URL derived from the current port with a copy action of its own, the
client token (see `mcp-server` "Client authentication") masked, with copy
and regenerate actions, and an installation-command generator (agent-type
picker + the exact command for that type + copy action), each persisting on
change. The installation command's agent-type picker SHALL offer Claude,
Codex, OpenCode, Gemini and Copilot — not Shell, which runs no MCP client to
register. Each installation command SHALL configure the client to send the
client token as a bearer `Authorization` header, using that client's own
syntax. Regenerating the token SHALL ask for confirmation, say that clients
registered with the old token stop connecting, and leave Knot-launched agents
unaffected. When the server has refused requests for a missing or invalid
credential in the last hour, the tab SHALL say how many, next to the
installation command.

The URL SHALL be the MCP endpoint an agent connects to, not the server's
root: it carries the `/mcp` path, and a URL without it is one an agent
cannot use.

#### Scenario: Toggling the MCP server persists

- **WHEN** the user turns off "Enable MCP server"
- **THEN** `mcp_server_enabled` is saved as `false` immediately

#### Scenario: Changing the port persists and updates the URL

- **WHEN** the user sets the port field to `9000`
- **THEN** `mcp_server_port` is saved as `9000` and the displayed URL
  updates to reflect port `9000`, keeping its `/mcp` path

#### Scenario: Copying the server URL

- **WHEN** the user clicks the copy action beside the URL
- **THEN** the system clipboard receives exactly the displayed URL,
  including its `/mcp` path

#### Scenario: Installation command matches the selected agent type

- **WHEN** the user selects "Codex" in the agent-type picker
- **THEN** the displayed command is the Codex-specific registration
  command containing the current server URL and the client token header

#### Scenario: Copy action copies the exact displayed command

- **WHEN** the user clicks the copy action
- **THEN** the system clipboard receives exactly the currently-displayed
  command text

#### Scenario: The token is masked until copied

- **WHEN** the MCP tab is shown
- **THEN** the client token is displayed masked, and the copy action puts
  the full token on the clipboard

#### Scenario: Regenerating the token

- **WHEN** the user confirms "Regenerate token"
- **THEN** a new token is stored in the Keychain, the old one is refused
  from then on, and the installation command shows the new token

#### Scenario: Refused clients are surfaced

- **WHEN** the server refused 3 requests for missing credentials in the last hour
- **THEN** the tab shows that 3 connection attempts had no valid token

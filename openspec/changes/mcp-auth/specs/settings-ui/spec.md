## ADDED Requirements

### Requirement: MCP tab manages the client token

The MCP tab SHALL show the client token (see `mcp-server` "Credential
kinds") masked, with copy and regenerate actions. Regenerating SHALL ask for
confirmation, say that clients registered with the old token stop
connecting, and leave Knot-launched agents unaffected. When the server
refused requests for a missing or invalid credential in the last hour, the
tab SHALL say how many.

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

## MODIFIED Requirements

### Requirement: MCP tab

The MCP tab SHALL show an "Enable MCP server" toggle
(`mcp_server_enabled`), a port field (`mcp_server_port`), the read-only
server URL with a copy action, and an installation-command generator: an
agent-type picker offering Claude, Codex, OpenCode, Gemini and Copilot (not
Shell), the exact command for that type, and a copy action. Each persists on
change. Each command SHALL send the client token as a bearer `Authorization`
header in that client's syntax. The URL SHALL carry the `/mcp` path.

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

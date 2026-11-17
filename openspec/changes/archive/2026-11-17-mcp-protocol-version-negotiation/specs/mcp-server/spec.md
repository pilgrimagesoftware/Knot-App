# Spec Delta

## ADDED Requirements

### Requirement: Protocol version negotiation

The server SHALL keep a list of the MCP revisions it implements. `initialize`
SHALL answer with the client's requested `protocolVersion` when it is on that
list, and with the newest listed revision otherwise, including when the
client names none.

#### Scenario: A supported version is echoed

- **WHEN** a client sends `initialize` asking for `2025-06-18`
- **THEN** the response's `protocolVersion` is `2025-06-18`

#### Scenario: An unsupported version gets the newest

- **WHEN** a client sends `initialize` asking for `2024-11-05`
- **THEN** the response's `protocolVersion` is the newest revision the server
  implements

### Requirement: Protocol version header

A request that carries an `Mcp-Session-Id` and an `MCP-Protocol-Version`
header naming a version the server neither implements nor has negotiated in
the past SHALL be refused with HTTP 400. A request with no session, or with
no header, SHALL NOT be refused for its version.

#### Scenario: An unknown version on a session

- **WHEN** a client on a session sends `tools/list` with
  `MCP-Protocol-Version: 2099-01-01`
- **THEN** the response status is 400

#### Scenario: A probe before initialize

- **WHEN** a client with no session sends `server/discover` with
  `MCP-Protocol-Version: 2026-07-28`
- **THEN** the request is answered rather than refused

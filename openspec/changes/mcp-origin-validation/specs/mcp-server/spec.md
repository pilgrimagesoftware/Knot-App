# Spec Delta

## ADDED Requirements

### Requirement: Origin validation

The server SHALL serve a request that sends no `Origin` header, or whose
`Origin` is an `http` or `https` origin on `localhost`, `127.0.0.1` or
`[::1]` at any port. Every other request SHALL be answered with HTTP 403 and
a JSON-RPC error response with no id. The rule SHALL apply to every route.

#### Scenario: An agent sends no Origin

- **WHEN** a client sends `initialize` with no `Origin` header
- **THEN** the request is served

#### Scenario: A loopback Origin

- **WHEN** a request arrives with `Origin: http://localhost:3000`
- **THEN** the request is served

#### Scenario: Any other Origin

- **WHEN** a request to `/mcp` or a hook endpoint arrives with
  `Origin: https://example.com` or `Origin: null`
- **THEN** the response status is 403 and the body is a JSON-RPC error with
  no id

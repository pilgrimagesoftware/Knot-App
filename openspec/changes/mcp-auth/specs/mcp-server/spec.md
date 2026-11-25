## ADDED Requirements

### Requirement: Client authentication

The server SHALL require a bearer credential issued by Knot on every route
except `/health`, sent as `Authorization: Bearer <credential>`. Knot SHALL
issue two kinds of credential, each 256 bits from the operating system's
CSPRNG:

- an agent credential per launched agent, held in memory only and discarded
  when that agent's session ends, identifying that agent;
- one client token for MCP clients the user registers by hand, stored in the
  macOS Keychain, persisting across launches until the user regenerates it,
  identifying an external caller.

The server SHALL compare presented credentials in constant time and SHALL
store only their digests. A request with a missing, malformed or unknown
credential SHALL receive HTTP 401 with `WWW-Authenticate: Bearer` and a
JSON-RPC error with no id, and SHALL change no state. A credential SHALL
never be written to a log, a URL or a process argument.

#### Scenario: No credential

- **WHEN** a client POSTs `initialize` to `/mcp` with no `Authorization` header
- **THEN** the response is 401 with `WWW-Authenticate: Bearer`, and no MCP
  session is created

#### Scenario: Unknown credential

- **WHEN** a request carries `Authorization: Bearer` with a value Knot did not issue
- **THEN** the response is 401 and nothing changes

#### Scenario: Health stays open

- **WHEN** a client GETs `/health` with no credential
- **THEN** the response is 200 and reveals no agent, session or credential data

#### Scenario: Agent credential ends with its agent

- **WHEN** an agent is closed and a request presents that agent's credential
- **THEN** the response is 401

#### Scenario: Regenerating the client token revokes the old one

- **WHEN** the user regenerates the client token and a client presents the previous token
- **THEN** the response is 401

#### Scenario: Credentials are not logged

- **WHEN** any authenticated or refused request is logged
- **THEN** the log entry names the principal (agent id, `external` or `none`)
  and the outcome, and contains no credential

### Requirement: Host validation

The server SHALL serve a request only when its `Host` header is
`127.0.0.1:<port>` or `localhost:<port>` for the port it listens on. Every
other request, including one with no `Host`, SHALL be answered with HTTP 403
and a JSON-RPC error with no id, before any credential is examined. The rule
SHALL apply to every route, `/health` included.

#### Scenario: A rebinding host name

- **WHEN** a request arrives with `Host: attacker.example:8767`
- **THEN** the response is 403, whatever credential it carries

#### Scenario: Loopback host

- **WHEN** a request with a valid credential arrives with `Host: localhost:<port>`
- **THEN** the request is served

## MODIFIED Requirements

### Requirement: Status endpoint

The system SHALL expose `GET /api/v1/agent/status` returning a JSON array with
one entry per agent: agent id, name, folder, state, agent-set status text,
registered flag, agent type, session id when present, and hook metadata when
non-empty. Keys SHALL be stably ordered. The endpoint SHALL require a
credential of either kind per "Client authentication"; without one it SHALL
answer 401 and return no agent data.

#### Scenario: Status reflects live agents

- **WHEN** two agents exist, one registered with a session id, and the request
  carries a valid credential
- **THEN** the array has two entries and the registered agent's entry includes
  its `session_id`

#### Scenario: Status without a credential

- **WHEN** a client GETs `/api/v1/agent/status` with no credential
- **THEN** the response is 401 and its body lists no agents

### Requirement: Origin validation

The server SHALL serve a request that sends no `Origin` header, or whose
`Origin` is an `http` or `https` origin on `localhost`, `127.0.0.1` or
`[::1]` at any port. Every other request SHALL be answered with HTTP 403 and
a JSON-RPC error with no id. The rule SHALL apply to every route. Passing
this check SHALL NOT by itself serve a request: "Host validation" and
"Client authentication" also apply.

#### Scenario: An agent sends no Origin

- **WHEN** a client sends `initialize` with no `Origin` header and a valid credential
- **THEN** the request is served

#### Scenario: A loopback Origin

- **WHEN** a request arrives with `Origin: http://localhost:3000` and a valid
  credential
- **THEN** the request is served; without a credential the response is 401

#### Scenario: Any other Origin

- **WHEN** a request to `/mcp` or a hook endpoint arrives with
  `Origin: https://example.com` or `Origin: null`
- **THEN** the response status is 403 and the body is a JSON-RPC error with
  no id

## ADDED Requirements

### Requirement: Client authentication

The server SHALL require a bearer credential issued by Knot (see "Credential
kinds") on every route except `/health`, sent as
`Authorization: Bearer <credential>`, and SHALL compare it in constant time.
A missing, malformed or unknown credential SHALL get HTTP 401 with
`WWW-Authenticate: Bearer`; a known credential on a route its kind does not
cover SHALL get 403. Either response SHALL carry a JSON-RPC error with no id
and change no state.

#### Scenario: No credential

- **WHEN** a client POSTs `initialize` to `/mcp` with no `Authorization` header
- **THEN** the response is 401 with `WWW-Authenticate: Bearer`, and no MCP
  session is created

#### Scenario: Unknown credential

- **WHEN** a request carries `Authorization: Bearer` with a value Knot did not issue
- **THEN** the response is 401 and nothing changes

#### Scenario: A hook credential on the MCP route

- **WHEN** a request to `/mcp` presents an agent hook credential
- **THEN** the response is 403 and no session is created

#### Scenario: Server info needs a credential

- **WHEN** a client GETs `/` with no credential
- **THEN** the response is 401

#### Scenario: Health stays open

- **WHEN** a client GETs `/health` with no credential
- **THEN** the response is 200 and reveals no agent, session or credential data

### Requirement: Credential kinds

Knot SHALL issue three kinds of credential, each 256 bits from the operating
system's CSPRNG. Per launched agent, held in memory and revoked when its
session ends: a tool credential, accepted on `/mcp` and the status GET, and a
hook credential, accepted only on the hook routes. One client token for
clients the user registers by hand, kept in the macOS Keychain until
regenerated, is accepted on every authenticated route as an external caller.

#### Scenario: Agent credential ends with its agent

- **WHEN** an agent is closed and a request presents that agent's credential
- **THEN** the response is 401

#### Scenario: Regenerating the client token revokes the old one

- **WHEN** the user regenerates the client token and a client presents the previous token
- **THEN** the response is 401

### Requirement: Credentials stay secret

The server SHALL store only digests of issued credentials. Knot SHALL NOT
write a credential to a log, a URL, or a command line it builds. Where an
adapter forwards the MCP headers to its own child processes is outside Knot's
control and SHALL be documented per adapter.

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

#### Scenario: Another host name

- **WHEN** a request arrives with `Host: other.example:<port>`
- **THEN** the response is 403, whatever credential it carries

#### Scenario: Another port

- **WHEN** a request arrives with `Host: 127.0.0.1:<a port other than the listening one>`
- **THEN** the response is 403

#### Scenario: Loopback host

- **WHEN** a request with a valid credential arrives with `Host: localhost:<port>`
- **THEN** the request is served

## MODIFIED Requirements

### Requirement: Status endpoint

The system SHALL expose `GET /api/v1/agent/status` returning a JSON array with
one entry per agent: agent id, name, folder, state, agent-set status text,
registered flag, agent type, session id when present, and hook metadata when
non-empty. Keys SHALL be stably ordered. The endpoint SHALL require a
credential of either kind per "Client authentication".

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
a JSON-RPC error with no id. The rule SHALL apply to every route. Passing it
SHALL NOT by itself serve a request: "Host validation" and "Client
authentication" also apply.

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

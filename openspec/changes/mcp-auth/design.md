# Design

> **Decision record candidate.** Adding authentication to the MCP control
> plane, and choosing two credential kinds (per-agent, in memory, plus a
> persistent client token) over a single per-launch token, is a lasting
> architectural decision. Record it as such when the change is accepted.

## Context

Current state, from the code at `ec977e25`:

- **Server.** `knot-mcp/src/server.rs` binds `127.0.0.1` on
  `mcp_server_port`. The default is 8767 in code (`knot-mcp/src/consts.rs`),
  while `mcp-server` "Local bind and configuration" says 8766; that
  disagreement is out of scope here.
- **Routes.** `server/routes.rs` mounts `/health`, `/`, `/mcp` (POST and GET)
  and the `/api/v1/agent/status` (GET and POST) and `/api/v1/agent/register`
  routes. The only guard on them is the Origin middleware (`origin.rs`).
- **Identity.** Identity comes from `?agent=<uuid>` (`routes.rs`
  `connection()`). `session.rs` `bind` replaces any earlier session for that
  agent. `knot-mcp-tools/src/identity.rs` refuses a call whose caller
  argument differs from the bound agent.
- **Agents.** ACP agents get the URL in `session/new` and `session/load`
  `mcpServers` (`knot-acp/src/client/mod.rs` `mcp_servers`), with
  `headers: []`. That only happens when the adapter declares
  `mcpCapabilities.http`; OpenCode and Gemini do not, and they get no entry.
- **External clients.** Users register them with the commands the MCP
  settings tab generates (`settings_window/panes/mcp.rs`
  `mcp_install_command`). Those commands carry no agent parameter, so the
  connections are unbound.
- **Hooks.** The scripts under `plugin/claude/scripts` and
  `plugin/codex/scripts` POST to the hook routes with `curl`. They read
  `SKWAD_AGENT_ID` and `SKWAD_URL` (default port 8766), which nothing in the
  Rust launch path sets. See Open questions.

## Goals

- A request that cannot present a credential Knot issued changes nothing and
  learns nothing beyond `/health`.
- A connection can act as an agent only by proving it is that agent. An
  agent's live session cannot be displaced by someone else.
- Knot-launched agents keep working with no user action, including across an
  app upgrade.
- Users can still point their own MCP clients at Knot.

## Decisions

### D1: Two credential kinds, not one shared token

A single per-launch token handed to every agent would authenticate "some
agent", but every agent would hold the token that impersonates every other
one. That leaves H2's problem in place. So:

| Credential | Scope | Lifetime | Storage | Principal |
|---|---|---|---|---|
| Agent credential | one agent | that agent's session in this app run | memory only | `Agent(id)` |
| Client token | the user's own clients | until regenerated | Keychain (`com.pilgrimagesoftware.knot.mcp-client-token`) | `External` |

The client token is persistent, not per launch. It is written into the
user's agent CLI configuration (`claude mcp add --scope user …`), and a token
that changes on every Knot restart would break every external client daily.
Regenerating it is the rotation and revocation path.

### D2: Credential format and comparison

- **Format.** 32 random bytes from `getrandom`, encoded as base64url without
  padding and prefixed with the kind (`knot_a_…` for agent, `knot_c_…` for
  client) so leaked credentials are recognisable to secret scanners. The
  bug-report scrubber gains the same patterns.
- **Storage and lookup.** The registry stores SHA-256 digests, not the
  credentials themselves. A presented credential is hashed, and the digest is
  compared to every stored digest with a constant-time comparison (`subtle`).
  There is no map lookup keyed by the secret. The number of credentials is
  small (one per agent plus one).
- **Logging.** Credentials are never logged. The MCP log records the
  principal (`agent <id>` / `external` / `none`) and the auth outcome.

### D3: Delivery to Knot-launched agents

- **ACP agents with HTTP MCP.** `mcpServers: [{ type: "http", name, url,
  headers: [{ name: "Authorization", value: "Bearer knot_a_…" }] }]` on both
  `session/new` and `session/load`. The URL keeps `?agent=<id>` as a
  non-secret hint, useful in logs and for the existing identity-error text.
- **ACP agents without HTTP MCP** (OpenCode, Gemini today). These have no
  automatic path today either; they rely on a user-registered client.
  Unchanged by this design, apart from that client now needing the client
  token. Adding a stdio bridge that carries the agent credential is a
  possible follow-up (see Open questions).
- **Hook scripts.** The adapter subprocess environment gets
  `KNOT_MCP_URL` and `KNOT_MCP_TOKEN` (the agent credential), and the
  scripts send `Authorization: Bearer $KNOT_MCP_TOKEN`. An environment
  variable is visible only to the same user (`ps -E` and the like), which is
  the same trust boundary as the Keychain item. It is not a command-line
  argument.
- **Shell agents.** They run no MCP client and get no credential. The
  `KNOT_AGENT` environment marker is unchanged.

### D4: Request handling order

1. **`Host` check.** `127.0.0.1:<port>` or `localhost:<port>` (port
   optional only when it is 80, which never applies). Otherwise 403.
2. **Origin check.** Unchanged rule.
3. **Credential.** `/health` skips this step. A missing or unknown
   credential gets 401 with `WWW-Authenticate: Bearer realm="knot"` and a
   JSON-RPC error with no id.
4. **Principal versus hint.** `Agent(x)` with `?agent=y` where x ≠ y gets
   403.
5. **Session binding** (below).

Host comes before credentials, so a DNS-rebinding page is refused without
learning whether a credential would have worked.

### D5: Session binding

- `Agent(x)` binds the connection to x. It replaces an earlier session for x
  **only if** that session was bound by `Agent(x)` too, which is the
  restarted-agent case and still works.
- `External` connections are unbound, as today. `register-agent` (or any
  tool naming `agentId`) from an `External` connection may act as x only if
  no `Agent(x)` session is live. Otherwise it is refused with the existing
  "already registered from another live session" error. This keeps today's
  manual-registration flow for agents without HTTP MCP.
- An `External` connection never evicts any session.

**Residual risk:** anyone holding the client token can act as an agent that
has no live agent-credential session. The token is user-scoped and the user
chose to hand it to that client, so this is accepted and documented.

### D6: Hook routes

- An `Agent(x)` credential sets `agent_id` to x. A body `agent_id` that
  differs gets 403.
- An `External` credential may post hooks for any agent that has no live
  agent-credential session. This covers hooks of a user-registered CLI.
- The routes are otherwise unchanged. Confining `transcript_path` is a
  separate follow-up (non-goal).

### D7: Status endpoint

`GET /api/v1/agent/status` requires a credential of either kind. Its
response is unchanged. The menu-bar and diagnostics consumers inside Knot
read the store directly, not this endpoint (to be verified in task 1.4).

## Migration

- **Knot-launched agents across an upgrade.** Agent credentials are
  in-memory, so every agent gets a new one when the upgraded app launches it
  or `session/load`s it. ACP sessions already end with the app process, so
  there is no running session holding an old URL. No action needed.
- **Hook scripts.** The scripts ship with the app (`plugin/`), so they and
  the server change together.
- **External clients registered before the upgrade** get 401 after it. To
  make that visible:
  - The server counts 401s by `User-Agent` (no credential material).
  - The MCP tab shows "N connection attempts without a valid token in the
    last hour" with a link to the install command.
  - The release notes and the docs MCP page say to re-run the install
    command.
- **No silent fallback.** There is no "allow unauthenticated" mode by
  default. Whether to ship a temporary, opt-in, off-by-default compatibility
  switch for one release is open question Q2.

## Open questions

- **Q1: Confirmation for MCP-created shell agents and worktrees.** Even with
  authentication, an authenticated agent (for example one following injected
  instructions) can call `create-agent` with `agentType: shell` and a
  `shellCommand`, or `create-worktree` at any absolute path. Options:
  - (a) require a UI confirmation showing the command and folder;
  - (b) allow it only for agents the user marks as orchestrators;
  - (c) leave it as is and rely on authentication.

  The recommendation is (a) for shell agents with a non-empty command, and
  for worktree destinations outside the repo's parent directory. **Maintainer
  decision.**
- **Q2: One-release compatibility switch** for unauthenticated external
  clients ("Allow MCP clients without a token", off by default, with a
  warning). The recommendation is not to ship it, because the 401 notice
  plus the install command covers the migration.
- **Q3: The hook scripts are on Skwad-era variables** (`SKWAD_AGENT_ID`,
  `SKWAD_URL`, default 8766) that the Rust launcher does not set. Are the
  hook routes in use today, and by what? If they are not wired, task 4.2
  updates the scripts to the `KNOT_*` variables as part of this change,
  rather than adding auth to a dead path.
- **Q4: Agents without HTTP MCP** (OpenCode, Gemini). Should Knot offer a
  small stdio-to-HTTP bridge that carries the agent credential, so these
  agents get an `Agent(x)` identity too? It is out of scope here; this is
  where the D5 residual risk lives.
- **Q5: Install-command header syntax per CLI.** It needs checking against
  current versions at implementation time: `claude mcp add --header`,
  `codex mcp add` (bearer-token environment variable), `gemini mcp add
  --header`, `agy mcp add`, the OpenCode config file, and Copilot.

## Risks

- **An adapter that ignores `mcpServers[].headers`.** It would fail to
  connect (401) instead of silently working unauthenticated. Task 3.2 checks
  each supported adapter live. An adapter that cannot send headers falls
  under Q4.
- **Keychain access prompts.** The client token is read once at startup. An
  ad-hoc-signed build may prompt the user on every new build. That is
  acceptable for developers, and it goes away with Developer ID signing.

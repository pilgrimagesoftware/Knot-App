# Design

> **Decision record candidate.** Adding authentication to the MCP control
> plane, with separately scoped credentials (a tool credential and a hook
> credential per agent, plus a persistent client token) rather than a single
> per-launch token, is a lasting architectural decision. Record it as such
> when the change is accepted.

## Context

Current state, from the code at `ec977e25`:

- **Server.** `knot-mcp/src/server.rs` binds `127.0.0.1` on
  `mcp_server_port`. The default is 8767 in code (`knot-mcp/src/consts.rs`),
  while `mcp-server` "Local bind and configuration" says 8766. That mismatch
  is out of scope here and tracked separately.
- **Routes.** `server/routes.rs` mounts `/health`, `/`, `/mcp` (POST and GET)
  and the `/api/v1/agent/status` (GET and POST) and `/api/v1/agent/register`
  routes. The only guard on them is the Origin middleware (`origin.rs`).
- **Identity.** It comes from `?agent=<uuid>` (`routes.rs` `connection()`).
  `session.rs` `bind` replaces any earlier session for that agent.
  `knot-mcp-tools/src/identity.rs` refuses a call whose caller argument
  differs from the bound agent.
- **ACP agents.** They get the URL in `session/new` and `session/load`
  `mcpServers` (`knot-acp/src/client/mod.rs` `mcp_servers`), with
  `headers: []`, when the adapter declares `mcpCapabilities.http`.
- **External clients.** They register with the commands the MCP settings tab
  generates (`settings_window/panes/mcp.rs` `mcp_install_command`). Those
  carry no agent parameter, so the connections are unbound.
- **Hooks.** The scripts under `plugin/claude/scripts` and
  `plugin/codex/scripts` POST to the hook routes, reading `SKWAD_AGENT_ID` and
  `SKWAD_URL` (default port 8766). Nothing in the Rust launch path sets these
  variables or installs the scripts (`knot-terminal/src/session.rs` notes that
  `plugin_root` is unused). The routes themselves are reachable regardless.

## Goals

- A request that cannot present a credential Knot issued changes nothing and
  learns nothing beyond `/health`.
- A connection can act as an agent only by proving it is that agent. An
  agent's live session cannot be displaced by anyone else.
- A credential that a process inherits through its environment can do as
  little as possible.
- MCP calls that would start a user-defined command, or create a worktree
  outside the repo's neighbourhood, need the user's confirmation.
- Knot-launched agents keep working with no user action, including across an
  app upgrade.
- Users can still point their own MCP clients at Knot.

## Decisions

### D1: Separately scoped credentials, not one shared token

A single per-launch token handed to every agent would authenticate "some
agent", and every agent could then act as every other one, which this change
must prevent. A credential in an agent's environment is also inherited by
every command the agent runs, so it should not be able to call tools. So:

| Credential | Prefix | Accepted on | Lifetime | Storage | Principal |
|---|---|---|---|---|---|
| Agent tool credential | `knot_a_` | `/mcp`, `GET /api/v1/agent/status` | one agent session in this app run | memory only | `Agent(id)` |
| Agent hook credential | `knot_h_` | `POST /api/v1/agent/register`, `POST /api/v1/agent/status` only | same as the tool credential | memory only | `AgentHooks(id)` |
| Client token | `knot_c_` | every authenticated route | until regenerated | Keychain (`com.pilgrimagesoftware.knot.mcp-client-token`) | `External` |

- A credential presented on a route its kind does not cover gets 403.
- The tool credential travels only in the MCP headers (D3). The hook
  credential is the only agent secret placed in an environment variable.
- The client token is persistent, not per launch. Users write it into their
  agent CLI's own configuration (`claude mcp add --scope user …`), and a token
  that changed on every Knot restart would break those clients daily.
  Regenerating it is the rotation and revocation path.
- **The client token is plaintext at rest in each CLI's configuration** (for
  example `~/.claude.json` after `claude mcp add --header`, or a shell
  variable that Codex's install command references). Knot's Keychain copy is
  protected; the copies the user hands to clients are not. This is accepted
  for clients the user chose, documented on the settings tab and the docs
  page, with regeneration as the remedy.

### D2: Credential format and comparison

- **Format.** 32 random bytes from `getrandom`, encoded as base64url without
  padding and prefixed by kind (table above), so leaked values are
  recognisable to secret scanners. The bug-report scrubber gains the same
  patterns.
- **Storage and lookup.** The registry stores SHA-256 digests, not the
  credentials. A presented credential is hashed, and the digest is compared
  with every stored digest using a constant-time comparison (`subtle`). There
  is no map lookup keyed by the secret, and the set is small (two per agent
  plus one).
- **Logging.** Credentials are never logged. The MCP log records the
  principal (`agent <id>`, `agent-hooks <id>`, `external` or `none`) and the
  outcome.

### D3: Delivery to Knot-launched agents, per adapter

- **Tool credential.** It goes in `mcpServers[].headers` (`Authorization:
  Bearer knot_a_…`) on both `session/new` and `session/load`. The URL keeps
  `?agent=<id>` as a non-secret hint (useful in logs and for attributing
  refused requests, D10).
- **Hook credential.** It goes in the adapter subprocess environment as
  `KNOT_MCP_TOKEN`, with `KNOT_MCP_URL`.
- **What Knot controls.** Knot puts neither credential in a URL, in any
  command line it builds, or in its logs.
- **What Knot does not control.** Where an adapter forwards the header to its
  own children is up to the adapter. It is recorded below and re-checked when
  adapters are updated (task 4.1).

**Per-adapter header feasibility (verified live on 2026-11-25, and re-run
with `scripts/acp-header-probe.py` for task 4.1 against the same versions).**

Method: each adapter was driven over ACP stdio with `session/new`, using an
`http` `mcpServers` entry whose header held a dummy bearer value, against a
loopback MCP stub. The stub recorded whether each request carried the header.
After 12 s, the argv of every descendant process (walked by parent PID) was
searched for the dummy value and for `--mcp-config`.

| Agent | Adapter (version) | `mcpCapabilities.http` | Header sent | Value in any argv | Notes |
|---|---|---|---|---|---|
| Claude | `claude-agent-acp` 0.112.0, SDK 0.3.331 | yes | yes, all requests | no | The adapter's `cliDelivery()` sends MCP servers in the CLI's stdin initialize message. It switches to `--mcp-config` argv only when `CLAUDE_CODE_EXECUTABLE` is set in the adapter's environment, or when the session's extra args name `settings` or `sdk-url`. Knot passes users' Claude options as `extraArgs`, so a user option can select the argv path (see below). |
| Codex | `codex-acp` 2.12.0 | yes | yes, all requests | no | |
| Antigravity | `agy` 1.5.17 (`acp --process-per-session`) | yes | yes on POST; **not** on the SSE `GET /mcp` | no | The stream GET gets 401. Re-checked in task 4.1 with the stub answering that GET with 401: agy went on to `tools/list` with the header, so it works on POST alone. The refused GET is still counted against agy's `?agent=` hint, so D10's panel notice also requires that the agent holds no live credential-bound session. |
| Copilot | `copilot --acp`, CLI 1.0.116 | yes | yes, all requests | no | Verified with `scripts/acp-header-probe.py`, probe run by the maintainer locally. No SSE `GET /mcp` was observed in the 12 s window. Copilot gets the tool credential like Claude, Codex and agy. |
| Gemini, OpenCode | — | no | — | — | No HTTP MCP in ACP, so no automatic path. They stay `External` (D5) until a stdio-bridge follow-up. |

**Claude argv path.** When Knot launches `claude-agent-acp` with
`CLAUDE_CODE_EXECUTABLE` in the environment it passes on, or with a user
option named `settings` or `sdk-url`, the agent's panel shows a notice and
the log records that this agent's MCP credential travels on the Claude CLI's
command line. Knot does not silently change the user's options.

**Shell agents** run no MCP client and get no credential. The `KNOT_AGENT`
environment marker is unchanged.

### D4: Request handling order

1. **`Host` check.** `Host` must be `127.0.0.1:<port>` or `localhost:<port>`
   for the listening port. Otherwise, or if `Host` is missing, the response
   is 403.
2. **Origin check.** The existing rule, unchanged.
3. **Credential.** Skipped for `/health`. A missing or unknown credential gets
   401 with `WWW-Authenticate: Bearer realm="knot"` and a JSON-RPC error with
   no id. A known credential on a route its kind does not cover gets 403.
4. **Principal versus hint.** `Agent(x)` or `AgentHooks(x)` with
   `?agent=y`, where x ≠ y, gets 403.
5. **Session binding.** See D5.

`/` (server info) is behind authentication like every route except `/health`.

### D5: Session binding

- `Agent(x)` binds the connection to x. It replaces an earlier session for x
  **only if** that session was also bound by x's current tool credential.
- `External` connections are unbound. `register-agent` (or any tool naming
  `agentId`) from an `External` connection may act as x only if no `Agent(x)`
  session is live. Otherwise it gets the existing "already registered from
  another live session" error. This keeps the manual-registration flow for
  Gemini, OpenCode and unverified adapters.
- `External` never evicts a session.

**Residual:** anyone holding the client token can act as an agent that has no
live tool-credential session. The token is user-scoped and handed to clients
the user chose. Accepted and documented. The stdio-bridge follow-up removes
this residual for Gemini and OpenCode.

### D6: Hook routes

- An `AgentHooks(x)` credential fixes `agent_id` to x. A body `agent_id` that
  differs gets 403.
- An `External` credential may post hooks only for an agent with no live
  tool-credential session.
- The hook credential is refused on `/mcp` and on the status GET (D1), so a
  process that inherits it from the agent's environment can report hook events
  for that agent and do nothing else.
- **Scripts.** `plugin/claude` and `plugin/codex` move from `SKWAD_*`/8766 to
  `KNOT_MCP_URL`/`KNOT_MCP_TOKEN` and send the `Authorization` header (task
  4.2). Installing them into agents remains out of scope, as it is today. The
  routes are authenticated either way.
- Confining `transcript_path` is a separate follow-up.

### D7: Status endpoint

`GET /api/v1/agent/status` needs a tool credential or the client token, with
an unchanged response. Before gating it, task 1.4 confirms that no internal
consumer calls it over HTTP.

### D8: Rotation on agent restart

Restart and Restart with New Conversation issue new tool and hook credentials
for the agent and revoke the previous pair **in one registry operation**,
under the registry lock, before the new adapter is spawned. No moment exists
in which both pairs are valid. A request from the old process that is still
shutting down gets 401. Closing an agent revokes its pair. Quitting Knot
drops the whole registry.

### D9: Confirmation for MCP-created commands and worktrees (in scope)

Authentication establishes *which* agent is calling, not that the user wants
the effect. These calls wait for a user confirmation that shows the requesting
agent, the command and the folder:

- `create-agent` with `agentType: shell` and a non-empty `shellCommand`;
- `create-worktree`, or `create-agent` with `createWorktree: true`, whose
  destination is outside the repository's parent directory after
  canonicalisation.

A denial, or no answer within the confirmation timeout, returns a tool error
and creates nothing. The same applies to any agent type whose launch command
becomes user-definable over MCP in future; none is today (`shellCommand` is
honoured only for `shell`). A shell agent with no command (a plain
interactive shell) and a worktree beside the repository need no confirmation.

### D10: Making silent header loss visible

If an adapter drops the header, its agent loses Knot's tools with 401s rather
than failing loudly. The server attributes a refused request to the agent
named in its `?agent=` hint (non-secret). That agent's panel then shows "Knot
tools unavailable: the MCP connection was refused", and the MCP tab's refused
counter lists it. `User-Agent` grouping for unattributed refusals keeps no
credential material.

## Migration

- **Knot-launched agents across an upgrade.** Credentials are in memory, and
  every agent gets a fresh pair when the upgraded app launches it or
  `session/load`s it. ACP sessions end with the app process. No action is
  needed.
- **External clients registered before the upgrade** get 401 afterwards. The
  MCP tab shows the refused count and the install command to re-run, and the
  release notes and the docs MCP page say the same.
- **No compatibility switch.** No "allow unauthenticated" mode, temporary or
  otherwise.

## Decided questions (maintainer, 2026-11-25)

- **Q1: Confirmation for MCP-created shell agents and worktrees.** In scope
  (D9).
- **Q2: Compatibility switch.** None.
- **Q3: Hook scripts and routes.** The routes are gated regardless. The
  scripts are migrated to `KNOT_*` in this change (D6).
- **Q4: Agents without HTTP MCP** (Gemini, OpenCode). They stay `External`.
  A stdio-bridge follow-up is filed separately; a bridge would carry the
  credential in its own process environment, with the same exposure as the
  hook credential.
- **Q5: Install-command header syntax per CLI.** Verified per CLI in task
  5.1. The client token is plaintext at rest in each CLI's configuration
  (D1).

## Risks

- **An adapter changes how it forwards headers.** The per-adapter table is
  re-checked when the adapter is updated (task 4.1 keeps the probe as a
  script), and D10 makes a regression visible.
- **agy's stream GET without a header.** See the table; task 4.1.
- **Keychain prompts on ad-hoc-signed builds.** The client token is read once
  per launch. Developer builds may prompt; signed releases will not.

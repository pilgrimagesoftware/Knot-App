# Proposal

## Why

Knot's MCP server is the control plane for every agent it hosts. Its tools
create agents and worktrees, relay messages between agents and change task
state. Today the server serves any client that can reach `127.0.0.1`, and it
takes an agent's identity from the `agent` query parameter in the URL. The
loopback bind and the Origin check keep remote hosts and ordinary web pages
out, but they do not tell Knot's own agents apart from any other program
running on the machine. Agents increasingly run tools in sandboxes, and a
sandbox that allows localhost also allows this server.

This change makes the server know who is calling it:

- every route except `/health` requires a credential Knot issued;
- an agent's identity comes from its credential, not from the URL;
- the `Host` header is checked alongside `Origin`.

## What Changes

- **Credentials.** Knot issues two kinds of bearer credential, each 256 bits
  from the OS CSPRNG:
  - an **agent credential** per launched agent, created at launch, held in
    memory only and discarded when the agent's session ends;
  - one **client token** for MCP clients the user registers by hand (an agent
    CLI in a plain terminal, or another tool). It is stored in the macOS
    Keychain and can be copied or regenerated from the MCP settings tab.
- **Every route requires one**, as `Authorization: Bearer <credential>`:
  `/mcp` (POST and GET), the hook routes, and `GET /api/v1/agent/status`. The
  exception is `/health`, which reveals nothing. A missing or unknown
  credential gets 401 with `WWW-Authenticate: Bearer`, and nothing changes.
  Comparison is constant-time.
- **Identity is bound to the credential.**
  - A connection presenting an agent credential *is* that agent. The URL's
    `agent` parameter becomes an optional hint that must match, or the
    request gets 403.
  - A connection presenting the client token is an *external* caller. It may
    register as an agent only if no agent-credential session holds that
    agent.
  - Binding never evicts a session held by an agent credential unless the new
    session presents the same credential.
- **Knot-launched agents get their credential automatically.**
  - ACP-launched agents receive it in the `headers` of the `mcpServers` entry
    on `session/new` and `session/load`.
  - Hook scripts receive it in an environment variable set on the adapter
    subprocess.
  - It never appears in a URL, a command line or a log.
- **Host allowlist.** Requests whose `Host` is not `127.0.0.1:<port>` or
  `localhost:<port>` get 403. The existing Origin rule stays.
- **The MCP settings tab** shows the client token masked, with copy and
  regenerate actions. The generated install command for each agent type
  includes the header.

### Non-goals

- Confining file paths that MCP tools and hooks accept (`transcript_path`,
  `display-markdown`, `create-worktree` destinations) and limiting read
  sizes. These are separate follow-ups. This change only ensures the callers
  are authenticated.
- Per-tool permissions (for example, an agent that may message but not
  create agents). This change establishes *who* is calling, which per-tool
  permissions would build on.
- TLS. The server stays plain HTTP on loopback.
- Changing the bind address or making the port random.

## Capabilities

### Modified Capabilities

- `mcp-server`: client authentication, `Host` validation, status endpoint
  behind authentication.
- `mcp-tools`: "A caller acts only as itself" and "A bound connection
  registers its agent" bind identity to the credential.
- `agent-launch-command`: the ACP MCP configuration carries the agent
  credential.
- `agent-hooks`: hook routes authenticate and take `agent_id` from the
  credential.
- `settings-ui`: the MCP tab manages the client token, and install commands
  include it.

## Impact

- `knot-mcp`:
  - an auth middleware (credential lookup, constant-time compare, 401/403);
  - a `Host` check next to `origin.rs`;
  - session binding keyed by principal;
  - status and hook routes behind authentication.
- `knot-mcp-tools`: `identity.rs` takes the caller's principal from the
  connection rather than from arguments alone.
- `knot-agent-launch` and `knot-acp`: the agent credential goes into
  `mcpServers[].headers` and into the adapter environment for hooks.
- `knot-core`: a credential registry (in memory) and Keychain storage for the
  client token.
- `knot` (settings window): token display, copy, regenerate and install
  commands.
- **Compatibility:**
  - MCP clients registered by hand before this release stop connecting (401)
    until the user re-runs the install command from the settings tab.
  - Knot-launched agents need no action, because a relaunch or
    `session/load` carries the new header.
  - See design.md, "Migration".
- Docs: `docs/site/latest/coordination.md` and the MCP setup page.

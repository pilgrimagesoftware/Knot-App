# Proposal

## Why

Knot's MCP server is the control plane for every agent it hosts. Its tools
create agents and worktrees, relay messages between agents, and change task
state. Today the server serves any client that can reach `127.0.0.1`, and it
takes an agent's identity from the `agent` query parameter in the URL. The
loopback bind and the Origin check keep remote hosts and ordinary web pages
out, but they do not tell Knot's own agents apart from any other program on
the machine. Agents increasingly run tools in sandboxes, and a sandbox that
allows localhost also allows this server.

This change makes the server know who is calling, and it asks the user before
an MCP call starts a user-defined command:

- every route except `/health` requires a credential that Knot issued;
- an agent's identity comes from its credential, not from the URL;
- the `Host` header is checked alongside `Origin`;
- MCP calls that would run a shell command, or create a worktree far from
  the repository, wait for the user's confirmation.

## What Changes

- **Credentials** (each 256 bits from the OS CSPRNG, stored as digests,
  compared in constant time):
  - **Agent tool credential**, one per launched agent, held in memory. It is
    sent only in the ACP `mcpServers[].headers` and accepted on `/mcp` and
    the status endpoint.
  - **Agent hook credential**, one per launched agent, held in memory. It is
    placed in the adapter environment as `KNOT_MCP_TOKEN` and accepted only
    on the two hook routes, so a process that inherits it cannot call tools.
  - **Client token**, for MCP clients the user registers by hand. It is
    stored in the macOS Keychain, can be copied or regenerated from the MCP
    settings tab, and is plaintext at rest in each client's own configuration
    (documented).
- **Every route except `/health` requires one**, as `Authorization: Bearer`.
  A missing or unknown credential gets 401 with `WWW-Authenticate: Bearer`;
  a credential on a route its kind does not cover gets 403. Nothing changes
  in either case.
- **Identity is bound to the credential.**
  - The URL's `agent` parameter becomes a hint that must match the
    credential.
  - A live agent session can be replaced only by the same agent's current
    credential.
  - The client token never displaces an agent.
  - Restarting an agent rotates its credentials atomically.
- **Per-adapter delivery is documented.** Knot never puts a credential in a
  URL, a command line it builds, or its logs. Where each adapter forwards the
  header was verified live and is recorded in design.md, including the
  conditions under which Claude's adapter would place it on the CLI's command
  line.
- **Host allowlist** (`127.0.0.1:<port>`, `localhost:<port>`), alongside the
  existing Origin rule.
- **Confirmation** before an MCP call creates a shell agent with a command,
  or a worktree outside the repository's parent directory.
- **Hook scripts** under `plugin/` move from the legacy `SKWAD_*` variables
  to `KNOT_MCP_URL`/`KNOT_MCP_TOKEN` and send the header.
- **MCP settings tab:**
  - the token is shown masked, with copy and regenerate actions;
  - install commands include the header;
  - refused connection attempts are counted, attributed to an agent where the
    URL names one.

### Non-goals

- Confining file paths that MCP tools and hooks accept (`transcript_path`,
  `display-markdown`), and limiting read sizes. These are separate follow-ups.
- Per-tool permissions beyond D9's confirmations.
- A stdio bridge giving Gemini and OpenCode agent credentials (follow-up).
  Until then they connect with the client token.
- Installing the hook scripts into agents.
- TLS, a different bind address, or a random port.
- No compatibility switch for unauthenticated clients.

## Capabilities

### Modified Capabilities

- `mcp-server`: client authentication, credential kinds and scopes, `Host`
  validation, status endpoint behind authentication.
- `mcp-tools`:
  - identity bound to the credential ("A caller acts only as itself",
    "A bound connection registers its agent");
  - confirmation for MCP-created shell commands and distant worktrees.
- `agent-launch-command`: credential delivery and rotation on restart.
- `agent-hooks`: hook routes accept the hook credential or the client token.
- `settings-ui`: the MCP tab manages the client token, and install commands
  include it.

## Impact

- `knot-mcp`:
  - auth middleware (credential lookup, constant-time compare, route scopes,
    401/403);
  - a `Host` check;
  - principal-keyed session binding;
  - refused-request attribution.
- `knot-mcp-tools`:
  - `identity.rs` takes the caller from the principal;
  - `create-agent` and `create-worktree` wait for confirmation per D9.
- `knot-agent-launch`, `knot-acp`, `knot-terminal`:
  - the tool credential goes into the `mcpServers` headers, the hook
    credential into the adapter environment;
  - credentials rotate on restart;
  - a notice appears when the Claude adapter would use its argv delivery.
- `knot-core`: the credential registry, and Keychain storage for the client
  token.
- `knot`: the settings tab, the confirmation dialog, and the
  tools-unavailable notice on the agent panel.
- `plugin/claude`, `plugin/codex`: scripts migrated to `KNOT_*`.
- **Compatibility:**
  - Clients registered by hand before this release stop connecting until the
    user re-runs the install command.
  - Knot-launched agents need no action.
- Docs: `docs/site/latest/coordination.md` and the MCP setup page.

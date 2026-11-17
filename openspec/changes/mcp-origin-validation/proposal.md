# Proposal

## Why

The MCP Streamable HTTP transport requires a server to validate the
`Origin` header on every incoming connection and to answer an invalid one
with 403 (MCP 2025-11-25, Transports, "Security Warning"; the same
requirement has stood since 2025-03-26). Knot's server did not read
`Origin`.

## What Changes

- Every route on the server (`/mcp`, the agent hook endpoints, `/health`,
  `/`) checks `Origin`. A request without one is served, as is one whose
  `Origin` is `http` or `https` on `localhost`, `127.0.0.1` or `[::1]` at
  any port. Anything else, including the opaque origin `null`, is answered
  with 403 and a JSON-RPC error with no id.
- Before choosing the rule, the headers each supported agent sends were
  captured through a logging proxy: Claude Code 2.1.311, Codex
  (`codex-mcp-client` 0.169.0), agy 1.5.9 over ACP and opencode 2.2.5 send no
  `Origin`. The hook scripts call the endpoints with `curl`, which sends none
  either.

## Impact

- `knot-mcp`: new `origin` module; `consts::LOOPBACK_ORIGIN_HOSTS`; a
  middleware layer on the router.
- Specs: ADDED `mcp-server` "Origin validation".

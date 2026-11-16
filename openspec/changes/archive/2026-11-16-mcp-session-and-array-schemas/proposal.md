# Proposal

## Why

Antigravity (agy) sessions got none of Knot's MCP tools (#431), even with
`agy acp --process-per-session` advertising `mcpCapabilities.http` and Knot
passing its server on `session/new`. Captured between agy 1.5.9 and Knot's
server, two faults on Knot's side:

1. agy's Go MCP client sends `server/discover` (MCP 2026-07-28) before
   `initialize`. Knot answered the sessionless probe with a fresh
   `Mcp-Session-Id`; agy sent that id on `initialize`; Knot answered with a
   different one, and agy closed the connection (`DELETE`) without
   `tools/list`. Every retry did the same.
2. With that fixed, the tools reached the model and Gemini rejected the
   whole turn: `describe-agents.capabilities` and `plan-tasks.tasks` are
   arrays with no `items`, which Gemini's function calling requires.

## What Changes

- Knot's MCP server issues a session id only in answer to `initialize`, or
  for a request that sent one (a known id is echoed; a stale one is still
  re-issued, the existing self-heal). A request that sends none and is not
  `initialize` is answered without one.
- Every array parameter in the tool catalog declares `items`.

## Impact

- `knot-mcp`: `server::connection` returns an optional session id.
- `knot-mcp-tools`: `with_items` for `describe-agents` and `plan-tasks`.
- Specs: ADDED `mcp-server` "A session id comes from initialize" and
  `mcp-tools` "Array parameters declare their items".
- Not changed: Knot still answers `initialize` with protocol version
  `2024-11-05`, and `GET` on any path still answers 200. Neither stopped agy
  (each was tested on its own and changed nothing); both are worth a
  separate look.

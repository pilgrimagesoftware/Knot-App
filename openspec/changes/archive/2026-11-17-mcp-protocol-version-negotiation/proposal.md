# Proposal

## Why

Knot's MCP server answered every `initialize` with protocol version
`2024-11-05`, whatever the client asked for (#442). That revision describes
the HTTP+SSE transport; the server speaks Streamable HTTP with
`Mcp-Session-Id`, which arrived in 2025-03-26. A client that trusts the
negotiated version can pick the wrong transport behaviour.

## What Changes

- The server keeps the list of revisions it implements, newest first:
  `2025-11-25`, `2025-06-18`. `initialize` echoes the client's requested
  version when it is on the list and answers with the newest otherwise
  (Lifecycle, "Version Negotiation").
- 2025-03-26 is left off because it requires a server to accept JSON-RPC
  batches, which Knot does not; 2024-11-05 because Knot does not serve its
  transport. Of 2025-06-18's changes, the one a server with Knot's features
  must act on is the `MCP-Protocol-Version` header. The rest are optional
  features or authorization, which Knot does not use.
- A request on a session whose `MCP-Protocol-Version` header names a version
  the server neither implements nor ever negotiated is refused with 400
  (Transports, "Protocol Version Header"). A request with no session is not
  checked: Claude Code and agy both send `server/discover` with
  `MCP-Protocol-Version: 2026-07-28` before they initialize.
  `2024-11-05` and `2025-03-26` stay accepted on the header so a client that
  kept its session across the upgrade is not cut off.

## Impact

- `knot-mcp`: `consts::PROTOCOL_VERSIONS`, `LEGACY_PROTOCOL_VERSIONS`,
  `PROTOCOL_VERSION_HEADER` replace `PROTOCOL_VERSION`; new
  `protocol_version` module; `rpc::dispatch` reads
  `params.protocolVersion`; `server` checks the header on `POST` and `GET
  /mcp`.
- Specs: ADDED `mcp-server` "Protocol version negotiation" and "Protocol
  version header".
- Not changed: the server still does not validate `Origin`, which every
  Streamable HTTP revision requires. That is a separate issue.

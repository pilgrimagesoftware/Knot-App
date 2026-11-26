---
id: ADR-0010
title: Authenticate the MCP server with separately scoped credentials
status: accepted
date: 2026-11-25
tags: [security, mcp, agents]
---

# ADR-0010: Authenticate the MCP server with separately scoped credentials

## Context

Knot's MCP server listens on loopback and, until this change, served any
request that passed its `Origin` check. A connection said which agent it was
through the `?agent=` URL parameter. Any local process could therefore call
Knot's tools as any agent, post hook events for it, or read every agent's
status. The OpenSpec change `mcp-auth` (`openspec/changes/mcp-auth/`) holds
the full design (D1–D10) and the maintainer's decisions on its open questions.

## Decision

Every route except `/health` requires a bearer credential issued by Knot, of
one of three kinds:

- **An agent tool credential** for each launched agent and each launch,
  accepted on `/mcp` and the status GET. It is delivered only in the ACP
  `mcpServers[].headers`.
- **An agent hook credential**, accepted only on the hook routes and only for
  its own agent. It is the only agent secret placed in an environment
  variable (`KNOT_MCP_TOKEN`).
- **One client token** for clients the user registers by hand. It is kept in
  the Keychain until regenerated, and the user's clients store it in plain
  text in their own configuration.

Identity comes from the credential. The URL's agent hint must agree with it.
The registry stores SHA-256 digests and compares them in constant time.
Issuing an agent's pair revokes the old pair under one lock. The client token
can never act for an agent Knot launched. MCP calls that create a shell
command or a distant worktree wait for the user's confirmation.

## Options considered

- **Separately scoped credentials** (chosen). A credential inherited through
  an agent's environment can only report that agent's hook events. The
  long-lived token is the one the user hands out, and regenerating it is the
  revocation path.
- **One per-launch token shared by every agent.** It would authenticate
  "some agent", so every agent could act as every other. It would also break
  hand-registered clients on every restart.
- **Keep identity in the URL and check only `Origin`.** This was the old
  behaviour. Any local process could act as any agent.

## Consequences

- Adapters must send the MCP entry's headers. Each is checked with
  `scripts/acp-header-probe.py`, and only checked types are given a
  credential (`knot_core::agent_type`'s `sends_mcp_headers`). The others reach
  Knot as external clients.
- Clients registered before the upgrade are refused until their installation
  command is run again. There is deliberately no compatibility switch.
- A client-token holder can still act as an agent Knot did not launch
  (Gemini and OpenCode, which have no HTTP MCP in ACP). A stdio-bridge
  follow-up would remove that.

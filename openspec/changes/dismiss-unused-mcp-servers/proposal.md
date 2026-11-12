# Proposal

## Why

An agent's account can offer MCP servers the user never set up. Claude
lists every claude.ai connector, connected or not, and each one Knot shows as
"Needs authentication". The MCP section then names them all as needing
attention, burying the servers that really do (#402).

Knot can't tell a connector the user never meant to use from one whose
sign-in lapsed, and treating every unauthenticated claude.ai connector as
unused would hide the second. So the user decides.

## What Changes

- A server needing attention gets a dismiss control. A dismissed server stops
  counting as needing attention, and is listed after the others under a
  "Not connected — dismissed" heading, muted and without its fix action, with
  a control that brings it back.
- Dismissal is kept by server name as a preference, across agents and
  launches.
- A dismissed server that connects rejoins the list as connected, so
  dismissing can never hide a server that is in use.
- Knot's own server cannot be dismissed.

## Impact

- `knot-core`: `Settings::dismissed_mcp_servers`, persisted with the
  preferences document. A document without the key reads as nothing
  dismissed.
- `knot`: `mcp_panel::rows::is_dismissed` decides which rows count as
  dismissed; `compose` lists them last, the header leaves them out of what
  needs attention, and the MCP pane draws the heading and the
  dismiss/restore control (`mcp_panel::dismiss`).
- `agent-mcp-status`: the collapsed-header requirement excludes dismissed
  servers, and a new requirement covers dismissing.
- `settings-persistence`: the preferences document holds the dismissed
  servers.

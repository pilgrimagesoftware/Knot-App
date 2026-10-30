# Proposal

## Why

Autopilot is one setting for the whole app. The port (archived
`2026-10-13-autopilot-port`) left a per-agent override out as a non-goal,
matching the Swift reference. Users now want to see which agents Autopilot is
reading, and to turn it off for one agent without turning it off for all
(#323).

## What Changes

- Each agent can have Autopilot turned off, from its context menu and the
  menu bar's Agents menu (Turn Off / Turn On Autopilot). The choice is durable;
  a duplicate or fork starts following Settings again.
- Autopilot never classifies a reply from an agent with it turned off. The
  check is in `spawn_autopilot`, where the hook and Panel triggers meet.
- An agent Autopilot reads shows an Autopilot detail line on its sidebar row
  and a propeller icon beside its status in the title bar.
- The menu items appear only while Autopilot is on in Settings, and never for
  shell agents.

## Impact

- `knot-core`: `SavedAgent::autopilot_off`.
- `knot-agents`: `Agent::autopilot_off`, `Agent::autopilot_applies`,
  `AgentStore::set_autopilot_off`.
- `knot-mcp-tools`: the per-agent gate in `spawn_autopilot`.
- `knot`: Agents/context menu entries, the sidebar line, the title bar icon.

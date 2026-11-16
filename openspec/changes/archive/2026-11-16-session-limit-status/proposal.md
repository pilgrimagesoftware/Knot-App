# Proposal

## Why

When a Panel agent's provider refuses a turn because its session or usage limit
is reached, the turn ends in an error and the agent goes back to Idle: a green
dot on an agent that cannot answer anything until the limit lifts. The error is
only in the conversation (#318).

## What Changes

- A turn that ends in an error reporting a session or usage limit puts the
  agent in Error - a red dot in its cell and the window's header - until the
  user sends another prompt.
- Wherever the status is named, it says "Session limit" instead of "Error":
  the header (with when it resets, when the provider said), the sidebar dot's
  tooltip, and the dashboard card, which shows the reset time on its own line.

## Impact

- `knot-agents`: `SessionLimit` (recognised from the error text),
  `Agent::session_limit`, `AgentStore::set_session_limit`; `set_state` clears
  it as the agent leaves Error.
- `knot`: `PanelState::session_limit`; `acp_status` reports Error for it; the
  status labels in the title bar, sidebar tooltip and dashboard.
- Terminal-mode agents are not covered: their limit message is terminal output,
  which is not parsed for status.

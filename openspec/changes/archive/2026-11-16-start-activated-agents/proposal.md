# Proposal

## Why

An agent can be activated without being started. The sidebar lights an agent
from its `activated` flag, and a direct message to an agent that is already
activated asks for nothing to be started, because the flag says it is running.
But the flag is set in more places than sessions are started:

- An agent created over MCP `create-agent` is `active`, and so activated, from
  birth. When its workspace is already open, nothing starts it.
- A companion inherits its owner's flag when it is created, and nothing starts
  it either.

Such an agent looked awake in the sidebar, took messages, and did nothing until
the user clicked its row (#316, #320).

## What Changes

- The workspace window's repaint poll starts every agent of its workspace that
  is activated and has no session, once per activation. A session that cannot
  start is not retried every tick, and deactivating or tearing an agent down
  forgets the claim, so the next activation starts it again.
- Deactivated and `passive` agents are untouched. The flag is the gate, and they
  do not carry it.

## Impact

- `knot`: a new `workspace_window::activation` module and one call in
  `repaint_poll_tick`.
- Spec: `agent-lifecycle`'s "Activation mode" gains the rule that an activated
  agent is a running one.

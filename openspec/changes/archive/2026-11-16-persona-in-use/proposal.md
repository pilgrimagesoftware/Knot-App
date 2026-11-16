# Proposal

## Why

A persona assigned to agents cannot be deleted, and its delete button's
tooltip said only how many agents use it. Finding them meant hunting through
every workspace (#365).

## What Changes

- The delete tooltip names the agents using the persona.
- A persona in use shows a "Used by" line listing those agents, each a link
  that opens the agent's workspace window with the agent selected (or raises
  it), so the persona can be changed there.

## Impact

- `knot`: the Personas pane; the library window holds the message store so
  it can open a workspace window (`WorkspaceWindow::open_agent`, #360).

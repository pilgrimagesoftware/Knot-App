# Proposal

## Why

Knot sometimes ran two `claude-agent-acp` adapters for one agent, both
resuming the same session (#382). Each copy read the agent's inbox, so a
message reached whichever copy read first. Both copies acted on the same
assignment. Only one had a pane, so the user could neither see nor close the
other.

The cause is teardown, not start. Every start already goes through
`ensure_panel_session`, which does nothing while the window holds a slot for
the agent. But teardown only stopped a slot that was `Ready`. A slot still
`Connecting` was swapped for a placeholder and dropped, and its connect task
kept running. It finished the handshake, published `Ready` into a slot no
window held, and sent the registration turn on an adapter with no pane. Then
the next start spawned the adapter that did get a pane.

Several things tear a session down while it is still connecting: a restart, a
deactivate, a plan step's fresh-context restart and closing the window. The
fresh-context case is the "started twice" noted in #384's review: a messaged
agent is started on its old conversation and restarted fresh before that
start connects. It was taken to be a wasted launch, but it was this leak.

## What Changes

- A panel slot that the window lets go of is **retired**, never dropped. A
  retired slot is final. A connect that is still running cancels when its slot
  is retired. If the adapter has started, it is killed. If the connection
  finished first, it is stopped. In both cases nothing is published and no
  session id is reported.
- Every teardown path (remove, deactivate, restart, retry, window close)
  releases the slot through one function, so a new path can't do it any
  other way.
- A start request for an agent whose slot is connecting or ready still
  attaches to that slot. This change keeps that behaviour and pins it with a
  test.

## Impact

- `knot`: `PanelSessionSlot` gains `Retired` and `retire()`. `connect_into`
  makes its connection in a future that a retire cancels.
  `WorkspaceWindow::retire_panel_session` is the one way a slot leaves the
  window.
- Spec: `agent-lifecycle` adds "A start attaches to the agent's running adapter" and "Teardown stops the adapter in any phase".
- No new dependencies. No public API outside the `knot` binary crate.

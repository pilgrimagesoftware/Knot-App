# Proposal

## Why

Restart Agent and Restart with New Conversation on an agent that is not
running did nothing visible (#311). A restart only tore the session down and
reset the session ids. Starting it again was left to whatever starts agents,
and nothing starts an agent that is not activated. The agent stayed exactly as
stopped as it was.

## What Changes

- Restarting an agent from its own row activates it first, so it starts: with
  the conversation the restart keeps, or a new one for Restart with New
  Conversation. Choosing the agent is an explicit request to run it, the same
  rule as selecting its row.
- A restart starts every restarted agent that is activated straight away, on
  screen or not, rather than waiting for its row to be drawn.
- Restart All still restarts only what is running. It does not start every
  agent the user had stopped.

## Impact

- `knot`: `WorkspaceWindow::restart_agent` for the row menu; `restart_agents`
  starts what it restarts.
- Spec: `agent-lifecycle`'s "Deactivating an agent" names a row restart as the
  third way a deactivated agent starts.

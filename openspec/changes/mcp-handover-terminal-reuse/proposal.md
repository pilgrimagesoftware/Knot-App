## Why

Acting on an MCP server that needs attention opens a shell companion that runs the agent's own MCP flow. Those terminals pile up (#443):

- every action created another one;
- one under a passive owner was never started, so it never exited and was never removed;
- the window tracked them only in memory, so after a restart nothing knew a saved "MCP" row was throwaway.

## What Changes

- A handover terminal is marked on its saved record with the server it was opened for.
- A repeated action for the same agent and server shows and starts the existing terminal instead of creating another. A different server gets its own terminal.
- The action shows and starts the terminal whatever the owner's activation mode: the user asked for it.
- A handover terminal left by an earlier run is not restored at launch. One left by a window that closed is removed when that workspace's window opens again, before anything starts.

## Capabilities

### Modified Capabilities

- `agent-mcp-status`: "A server needing attention delegates to the agent's own flow" gains reuse, start-on-action and cleanup.

## Impact

- `knot-core`: `SavedAgent.mcp_handover` (optional, omitted when unset; older records load without it).
- `knot-agents`: `Agent.mcp_handover`, `CreateOptions.mcp_handover`, `AgentStore::open_mcp_handover`, `mcp_handover_terminal`, `mcp_handover_owner`, `remove_mcp_handovers`.
- `knot`: the MCP panel action, the exit hook and window open use the store's marker; the window's `mcp_handover_terminals` map is gone; `build_agent_store` drops leftover terminals.

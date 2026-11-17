## 1. Marker

- [x] 1.1 `SavedAgent.mcp_handover` / `Agent.mcp_handover` / `CreateOptions.mcp_handover`, round-tripped by `convert`, not copied by duplicate or fork
- [x] 1.2 Older records load unmarked; unmarked records don't write the field

## 2. Reuse

- [x] 2.1 `AgentStore::open_mcp_handover` returns the existing terminal for the same owner and server, or creates one
- [x] 2.2 The MCP panel action goes through it and reveals the terminal it gets back
- [x] 2.3 The exit hook finds the owner from the marker; the window's in-memory map is removed

## 3. Cleanup

- [x] 3.1 `build_agent_store` drops leftover handover terminals
- [x] 3.2 Opening a workspace window removes its handover terminals before starting sessions, re-picks a selection that pointed at one, and persists

## 4. Tests

- [x] 4.1 Store: a repeated action keeps one terminal; another server gets its own; companions can't own one; ordinary companions aren't handovers; marker survives a reload; removal takes only handover terminals
- [x] 4.2 Startup drops a saved handover terminal and keeps the owner's ordinary companion
- [x] 4.3 Opening the workspace removes a leftover terminal, re-selects, and persists the removal

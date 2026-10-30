# Proposal

## Why

`set-status` writes an agent's status text on the MCP server's thread, which
has no GPUI context to notify from, and nothing in the window's repaint poll
noticed the write. The text reached the sidebar, title bar, dashboard and the
other places it's drawn only when something unrelated repainted the window.
That usually happened for an agent mid-turn and almost never for an idle one,
and the worst case was one agent setting another's status (#17).

## What Changes

- `AgentStore` keeps a revision counter, moved when an agent's status text or
  state actually changes (a write repeating the current value moves nothing).
- Each workspace window remembers the revision it last drew, and its repaint
  poll repaints when the store's has moved. It's a counter rather than a cleared
  flag, so every window over the shared store sees the change, not just the
  first to poll.
- The Command Center, which had no poll of its own, gets a small one that
  makes the same comparison, so its cards update too.

## Impact

- `knot-agents`: `AgentStore::revision`, moved by `set_status_text` and
  `set_state`.
- `knot`: `store_revision::revised`, `WorkspaceWindow::drawn_store_revision`
  and one term in `repaint_poll_tick`; `CommandCenterWindow`'s
  `drawn_store_revision` and its store poll.
- Spec: `mcp-tools`' `set-status` requires the new text to be drawn promptly.
- The artifact panes keep their own per-agent comparison, because it also
  reconciles the panel's expanded state, which a store-wide counter can't do.

# Proposal

## Why

After a scheduled start opened a workspace's window (#404), opening the
workspace from the Workspace Manager could leave every agent running twice:
the first window's adapters stayed alive beside the second window's, and
both acted on the same conversations (#456).

A workspace window's adapters are torn down in `WorkspaceWindow`'s `Drop`.
The window's repaint poll held its view strongly, in a loop that never ended,
so a closed window's view was never released: its adapters kept running and
its poll kept delivering inbox nudges to them. The registry correctly
dropped the closed window, so the next open made a new one and started
every agent again.

The scheduled window also never appeared in the Window menu. It opens
without focus, never becomes the active window, and only the active window
rebuilt the menu.

## What Changes

- The repaint poll holds its view weakly and ends with it, so closing a
  workspace window releases the view and stops its adapters.
- An app-wide check refuses to start an adapter for an agent that already
  has a live one (connecting or connected) in any window. A restart, which
  retires the old slot first, is not refused.
- A scheduled start that opens a window rebuilds the Window menu, so the
  window is listed at once.

## Impact

- `knot`: `workspace_window::repaint::spawn_repaint_poll` takes a
  `WeakEntity`; new `panel_session::live_adapters`; `ensure_panel_session`
  claims before it starts anything; `scheduled_starts::tick` refreshes the
  menu bar.
- Specs: ADDED `window-lifecycle` "Closing a workspace window stops its
  agents", "An agent runs one adapter at a time" and "A window opened by a
  schedule is listed".

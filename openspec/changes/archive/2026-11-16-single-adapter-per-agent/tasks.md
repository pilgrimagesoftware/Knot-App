# Tasks

## 1. Retire, don't drop

- [x] 1.1 Add `PanelSessionSlot::Retired` and `retire()`. `Connecting` holds
      the retirement signal, so replacing the variant fires it
- [x] 1.2 `connect_into`: race the connection (`prepare`) against the signal;
      publish and report the session id only while the slot is still
      `Connecting`, checked under its lock; stop a connection that lost
- [x] 1.3 Route remove, deactivate, restart, retry and window close through
      `WorkspaceWindow::retire_panel_session`

## 2. Tests

- [x] 2.1 Retiring a connecting slot kills the adapter it was starting, and
      publishes and reports nothing (real `sh` adapter, checked by pid)
- [x] 2.2 The race: a retire swept across the whole connect on a
      multi-threaded runtime leaves the slot retired and no adapter running
- [x] 2.3 A connect into an already-retired slot spawns nothing
- [x] 2.4 Restart, deactivate and a fresh-context restart (#384) each retire
      the connecting slot; a start while connecting attaches to it
- [x] 2.5 Mutation-check: with the signal ignored and the publish unchecked,
      2.1 and 2.2 fail; with teardown swapping in a placeholder, 2.4's three
      teardown tests fail

## 3. Gate

- [x] 3.1 `make` (fmt-check, size-check, declared-check, clippy, test, build)
- [x] 3.2 `openspec validate single-adapter-per-agent --strict`

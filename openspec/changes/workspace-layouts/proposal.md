# Proposal

## Why

A Rust workspace window shows one agent. The Swift app shows up to four at
once: side by side, stacked, three panes, or a 2×2 grid. The Rust app
already persists that arrangement (`layout_mode`, `active_agent_ids`,
`focused_pane_index`, `split_ratio`, `split_ratio_secondary` in the
per-workspace UI state). It also already promises it: `agent-lifecycle`
says creating a companion "enters a split layout pairing owner and
companion", and that removal "collapses split panes". The app stores a
layout it cannot draw and breaks two requirements it claims to meet (#8).

## What Changes

- A workspace window's content area shows one of five layouts: single, split
  vertical, split horizontal, three-pane, or four-pane grid. Each pane shows
  one agent's terminal, conversation panel or stopped view.
- Draggable dividers between panes. Each ratio is clamped to 0.25–0.75 and
  saved when the drag ends.
- A Layout menu in the content header switches modes. It is shown when the
  workspace has at least two agents.
- One pane is focused. The focused pane's agent is the window's selected
  agent: the sidebar highlights it, and the menus, shortcuts, git panel and
  artifact panel act on it.
- Sidebar selection follows the Swift rules: show a companion layout, focus
  an agent already in a pane, or replace the focused pane.
- Creating a companion opens it next to its owner. Removing an agent
  collapses the layout.
- A terminal keeps its grid size while a divider is dragged, and is resized
  once when the drag ends, so the PTY is not resized on every frame.
- The persisted layout is restored as saved and checked on load.

### Carried over from the Swift app

Read from `skwad` (`Views/ContentView.swift`, `Models/AgentManager.swift`,
`Views/Terminal/SplitDividerView.swift`), not inferred:

- The five modes and their pane geometry. In three-pane, pane 0 is the
  full-height left column and the right column is split. In the grid, both
  dividers run the full width or height.
- The 0.25–0.75 clamp on every divider. Terminals resize on drag end.
- The Layout menu's fill and trim rules.
- Sidebar selection, including ⌘-click to skip the companion layout.
- Companion creation stepping single → split → three-pane → grid. In a full
  grid, the new agent replaces another agent's pane, starting from the last.
- Removal collapsing grid → three-pane → split → single.
- A dimmed overlay on unfocused panes.

### Deliberately dropped or changed

- **Detached workspace window**: dropped. In Rust every workspace already
  has its own window (`window-lifecycle`, "A workspace has at most one
  window"), so there is nothing to detach. The stored `is_detached` stays
  as opaque data.
- **Launch override**: Swift replaces the current workspace's saved layout
  with a companion layout at launch. Here the saved layout is restored.
- **Deferred saves**: Swift saves layout changes only when something else
  saves. Here every layout change is persisted; a divider's ratio is saved
  on drag end.
- **Four-pane with three agents**: Swift can enter a grid with an empty
  fourth slot. Here a mode is offered only when the workspace has enough
  agents to fill it, so no pane is ever empty.
- **No bounds check on the focused pane**: Swift can index past the end of
  its active list. Here the focused pane is always clamped to the panes
  that exist.
- **Clicking a companion that is shown**: Swift checks "pane 0's agent has
  companions" before "already shown", so the click shows the agent alone.
  Its sidebar has no companion rows, so that path never runs there. Rust
  lists companions as rows, so an agent already shown is focused first.
- **Panel-mode agents**: Swift panes only hold terminals. A Rust pane holds
  whatever the agent shows: its terminal, its conversation panel, or the
  stopped view.

### Non-goals

- Keyboard shortcuts or View menu items for layouts. The Swift app has none
  either.
- Dropping an agent onto a pane.
- Per-pane git or artifact panels. Both stay window-level, following the
  focused agent.
- Any change to the Command Center's grid.

## Capabilities

### New Capabilities

- `workspace-layouts`: the panes a workspace window shows. Covers the
  layout modes and their geometry, the dividers, the Layout menu, the
  focused pane, sidebar selection into panes, the layout effects of
  creating companions and removing agents, resizing on settle, and
  restoring the saved layout.

### Modified Capabilities

- `terminal-rendering`: "Selected agent shows a live terminal surface"
  currently says every other agent's surface is hidden. It changes to cover
  every agent in a pane. "Surface resizes with its container" gains the
  rule that a surface holds its grid during a divider drag and resizes when
  the drag ends.
- `agent-lifecycle`: "Shell companions" and "Agent removal" name the layout
  effect but not the rules. They now defer to `workspace-layouts`, which
  also covers a companion created while the layout is already split or
  full. "Shell agent removal on process exit" is satisfied as written,
  since it follows the same removal sequence.

`agent-list-ui` ("New Shell Companion from the context menu": "the two
appear in a split layout") is satisfied as written. It defers to
`agent-lifecycle`, and from a single-pane layout the result is exactly
owner and companion side by side. `settings-persistence` ("Per-workspace UI
state is its own document") already stores every field this change reads,
with the same defaults. It is unchanged.

## Impact

- `knot-core`: a layout-mode enum replaces the opaque `layout_mode` string.
  It is stored as the Swift names `single`, `splitVertical`,
  `splitHorizontal`, `threePane`, `gridFourPane`, and an unknown value reads
  as `single`. Normalising the restored layout.
- `knot-agents`: the layout rules (select, companion, remove, mode change)
  live with the store's workspace UI state, wiring the setters that are
  marked `UNWIRED` today.
- `knot`: the workspace window's content area (`render/content.rs`) gets a
  layout branch in new modules beside it. `render/mod.rs` is at 655 of 700
  lines and takes nothing. Also a divider element, the Layout menu, the
  focused-pane wiring to `selected_agent`, and the hold on terminal sizes
  during a drag.
- Docs: `docs/site/` gains a layouts section.
- No new dependencies.

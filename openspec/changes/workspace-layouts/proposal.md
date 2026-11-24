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
- Sidebar clicks and the agent number shortcut apply four ordered rules
  (R1–R4): show a companion layout, focus an agent already in a pane, show
  the agent alone after a companion layout, or replace the focused pane.
  These are Swift's rules with two of them swapped; see below.
- Creating a companion opens it next to a shown owner. Removing an agent, or
  moving it to another workspace, collapses the layout.
- A terminal keeps its grid size while a divider is dragged, and is resized
  once when the drag ends, so the PTY is not resized on every frame.
- The persisted layout is restored as saved and checked on load.

### Carried over from the Swift app

Read from `skwad/Skwad`, not inferred (AM = `Models/AgentManager.swift`,
CV = `Views/ContentView.swift`):

- The five modes and their pane geometry (CV:599-639). In three-pane, pane 0
  is the full-height left column and the right column is split. In the grid,
  both dividers run the full width or height.
- The 0.25–0.75 clamp on every divider (CV:434, 465, 486).
- R1, R3 and R4 of the sidebar click (AM:1096-1124). ⌘ skips only R1
  (`SidebarView.swift`:78-79).
- The companion layout for zero to three companions (AM:986-1001).
- A new companion stepping single → split → three-pane → grid while its owner
  is shown. In a full grid, it replaces the last pane that is not the
  owner's (AM:1024-1079).
- Removal collapsing grid → three-pane → split → single (AM:737-772). Owners'
  companions are removed first (AM:719-723), which `agent-lifecycle`
  already requires.
- A dimmed overlay on unfocused panes, black at 0.1 opacity
  (`Theme.swift`:15).

### Deliberately dropped or changed

Each item below is a divergence from the Swift app, with the reason.

- **Terminals hold their size during a drag** (new). In Swift a drag sets
  the ratio live (CV:428-435), so panes and terminals relay out on every
  frame. Drag end only adds a resize nudge (CV:436-440). Here a terminal
  keeps its grid until the drag ends and is resized once. #8 asks for this:
  a PTY resize per frame makes every program in it redraw on every frame.
- **Layout menu fill and focus** (changed, CV:817-887). Swift fills a split
  only from exactly one shown agent, fills three-pane and grid only below
  three, never moves `focusedPaneIndex`, and takes Single's agent at an
  unchecked index. Here every mode fills any shortfall in sidebar order, and
  focus stays on its agent or else falls to pane 0. That way a mode always
  shows as many agents as it has panes, and focus always points at a pane.
- **Four-pane with three agents** (dropped). Swift can enter a grid with an
  empty fourth slot (CV:876-887). Here a mode is offered only when the
  workspace has enough agents to fill it, so no pane is ever empty.
- **Sidebar click order** (changed). Swift checks R3, "pane 0's agent has
  companions", before R2, "already shown" (AM:1103-1116). Its sidebar has no
  companion rows (`SidebarView.swift`:54), so a shown companion is never
  clicked there. Rust lists companion rows, and clicking a shown companion
  must focus it, not reset the layout.
- **Row highlight** (changed). Swift highlights the owner's row when a
  companion is focused (AM:391-394), because the companion has no row. Here
  the focused agent's own row is highlighted.
- **Agent number shortcut** (changed). Swift's `selectAgentAtIndex` counts
  only non-companion agents and focuses a shown agent's pane
  (AM:1153-1168). Here the shortcut counts the sidebar's rows as
  `keybindings` already specifies, and acts as a plain click on that row.
  Swift's next/previous-agent cycling (AM:1126-1151) is not ported; the port
  has no such shortcut (`app-menu`, "The View menu lists the navigation
  shortcuts").
- **Companion while its owner is not shown** (changed). Swift's MCP path
  re-lays out only when the owner is shown (`MCP/AgentCoordinator.swift`:613).
  Its UI paths (`AgentSheet.swift`:614, AM:830) step the current layout even
  when the owner is not in it, which can show a companion without its owner.
  Here a companion the user creates shows owner and companion as a split. A
  companion created any other way leaves the layout alone, because the user
  may be looking at something else.
- **Moving an agent to another workspace** (changed, AM:926-945). Swift only
  strips the id from the source's active list, which can leave a split with
  one agent, and never touches the target. Here the source collapses as on
  removal, and the target shows the agent only if it showed nothing, as the
  port's store already does today.
- **Detached workspace window** (dropped). In Rust every workspace already
  has its own window (`window-lifecycle`, "A workspace has at most one
  window"), so there is nothing to detach. The stored `is_detached` stays
  as opaque data.
- **Launch override** (dropped). Swift replaces the current workspace's
  saved layout with a companion layout at launch (AM:86-90). Here the saved
  layout is restored.
- **Deferred saves** (changed). Swift saves layout changes only when
  something else saves (AM:187-188). Here every layout change is persisted,
  and a divider's ratio is saved on drag end.
- **Focused-pane bounds** (changed). Swift can index past the end of its
  active list (AM:1121). Here the focused pane is always clamped to the
  panes that exist.
- **Panel-mode agents** (new). Swift panes hold terminals only. A Rust pane
  holds whatever the agent shows: its terminal, its conversation panel, or
  the stopped view.

### Non-goals

- Keyboard shortcuts or View menu items for layouts. Swift has none either:
  its layout toggle is a menu only (CV:815).
- Next/previous-agent cycling shortcuts. See above.
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

# Design

## Context

See proposal.md for the motivation. Swift citations use the proposal's
abbreviations (AM, CV, relative to `skwad/Skwad`). Current state, from the
code:

- `WorkspaceUiState` (`knot-core/src/settings/records.rs`) already carries
  the following fields, and `settings-persistence` already persists them in
  their own document, written often:
  - `layout_mode: String`, default `"single"`, opaque, uninterpreted
  - `active_agent_ids`
  - `focused_pane_index: i32`
  - `split_ratio: f64`, default 0.5
  - `split_ratio_secondary: Option<f64>`
- `AgentStore` (`knot-agents`) has setters for these. Three are marked
  `UNWIRED`. `create` seeds `active_agent_ids` with a workspace's first
  agent. `remove` and `move_to_workspace` drop an id from
  `active_agent_ids`, but leave the mode and the ratios alone.
  `create_shell_companion` does not touch the layout.
- `WorkspaceWindow.selected_agent: Option<Uuid>` drives everything:
  - `render/content.rs` (313 lines) draws one pane from it: stopped view,
    panel, or `terminal_panes[id].view`.
  - `with_side_panels` wraps that pane in an `h_resizable` with the git and
    artifact panels.
  - Menus, shortcuts and the sidebar highlight all read it.
- gpui-terminal (external, `v0.2.2`) fits its grid to its bounds in
  `prepaint` on every frame (`view/render.rs`, `view/entity.rs` `fit`) and
  resizes the PTY whenever the cell count changes. Nothing in Knot calls
  resize.
- gpui-base's `ResizablePanelGroup` sizes panels in pixels, one group per
  axis. It emits `ResizablePanelEvent::Resized` from `done_resizing`, on
  drag end.
- `render/mod.rs` is at 655 of 700 lines. The limit that applies is 700,
  the one `make size-check` enforces (`RUST_FILE_LINE_LIMIT`). The 500 in
  `CLAUDE.md` is stale, and fixing that line is outside this change.

## Goals / Non-Goals

**Goals:**

- One place decides the layout rules: select, mode change, companion,
  removal, load. Those rules can be tested without a window.
- `selected_agent` stays the value every existing reader uses. It always
  equals the focused pane's agent.
- No PTY resize while a divider is being dragged.

**Non-Goals:**

- Changing gpui-terminal or gpui-kit.
- Reworking the side panels. They wrap the whole layout area, keyed on the
  focused agent, as they wrap the single pane today.

## Decisions

### 1. `LayoutMode` becomes an enum in `knot-core`

`layout_mode` changes from a `String` to `LayoutMode { Single,
SplitVertical, SplitHorizontal, ThreePane, GridFourPane }`. It serializes
to the Swift names already on disk (`single`, `splitVertical`, ...). A
tolerant deserializer reads an unknown value as `Single` once, at load, per
the closed-vocabulary rule in `rust-structure.md`. `pane_count()` lives on
the enum.

The records test that round-trips `"grid"` changes to a real mode.

- Alternative: keep the string and match on it where it is used. Rejected:
  a misspelled mode would be indistinguishable from `single`, which is the
  #224 defect.

### 2. Layout rules are pure functions in `knot-agents`

A new `store/layout.rs` holds functions over the workspace's UI state, its
agent order and a companion lookup:

- `select(agent, skip_companions)`
- `set_mode(mode)`
- `add_companion(owner, companion, origin)`, where `origin` is `User` or
  `Other` and decides the owner-not-shown case
- `remove(agent)`
- `normalize()`

Each returns the new state. `AgentStore` calls `remove`/`add_companion`
from its existing `remove`, `move_to_workspace` and
`create_shell_companion`, so every caller of those (UI, MCP, process exit)
gets the layout effect without remembering to. The UI calls `select` and
`set_mode`. Each spec scenario becomes a unit test against these functions.

- Alternative: put the rules in the workspace window. Rejected: the window
  is not the only remover (MCP, process exit), and a rule spread across
  call sites is what left `active_agent_ids` half-maintained today.

### 3. `selected_agent` is written from the layout, not alongside it

The window keeps `selected_agent` because dozens of readers use it. It has
one writer after any layout change, which sets it to
`active_agent_ids[focused_pane_index]` and does what `select_agent` does
today (activation, persistence). The existing `select_agent` entry points
route through `layout::select`.

- Alternative: replace the field with a computed accessor. Better in the
  end, but it touches every reader in one change. It is left as a
  follow-up.

### 4. Panes are placed by ratio, not by nested resizable groups

The layout area is a `relative()` container. Panes and dividers are
absolutely positioned from two pure functions,
`pane_rects(mode, r, s, size)` and `divider_rects(...)`, both unit-tested.

- Alternative: nested `h_resizable`/`v_resizable` groups. Rejected for
  three reasons:
  - The grid's horizontal divider must be one line across both columns.
    Nested groups give each column its own divider.
  - The persisted value is a ratio clamped to 0.25–0.75 of the whole area.
    The groups store pixel sizes per panel.
  - The three-pane horizontal divider spans only the right column, and that
    can't be expressed with a shared ratio.

### 5. A divider is a small element; the drag state lives on the window

Each divider has a 12 px hit area centred on a 1 px line, and a
`ResizeLeftRight` or `ResizeUpDown` cursor. Mouse down records
`DividerDrag { divider, start_ratio, start_mouse, held_sizes }` on the
window. Window-level mouse move sets the clamped ratio and calls
`cx.notify()`. Mouse up, or a mouse-up anywhere outside the window, ends
the drag. Ending the drag clears the drag state, persists the ratios
through `persist_workspace_ui`, and lifts the size hold (decision 6). The
pattern follows gpui-base's resizable, which uses a drag marker plus
`done_resizing`.

### 6. Terminals hold their size during a drag by holding their container

When a drag starts, each terminal pane's pixel size is recorded in
`held_sizes`. While the drag lasts, the terminal's `TerminalView` sits in
an inner div fixed at that size, with overflow hidden, inside the pane's
moving bounds. gpui-terminal's `fit` sees the same bounds every frame and
does not resize. On release the hold is dropped, so the next prepaint fits
once, and the PTY gets one resize.

During the drag, a pane that grows shows its background beyond the old
grid, and a pane that shrinks clips it. This is new behaviour, not a port.
The Swift app relays its terminals out on every drag frame and only nudges
them again on drag end (CV:428-440). #8 asks for one resize on settle, so
that a full-screen program isn't redrawn on every frame of a drag.
Conversation panels are not held: they have no PTY and reflow live.

- Alternative: a "pause fitting" flag on gpui-terminal's view. Cleaner, but
  gpui-terminal is a separate MIT repo shared with Fernrohr. It can replace
  this later without changing behaviour.

### 7. Module placement

- New `workspace_window/render/layout.rs`: geometry and pane assembly.
- New `workspace_window/render/divider.rs`.
- New `workspace_window/render/layout_menu.rs`.
- New `workspace_window/layout.rs`: the window's layout actions, divider
  drag and the hold.
- `render/content.rs` replaces its single-pane body with a call into
  `layout.rs`. In single mode that call renders exactly today's pane, so
  single-agent behaviour is unchanged by construction.
- `render/mod.rs` gains only `mod` lines.
- Per-pane rendering factors today's stopped/panel/terminal branch into one
  `render_agent_pane(id)` that both modes use.

### 8. Showing an agent in a pane activates it

Placing an agent in a pane does what selecting it does today: it activates
the agent (`set_activated(id, true)`). A pane therefore shows a live agent,
not the stopped view, matching single mode. A mode change that fills new
panes can start up to three agents.

- Alternative: show filled panes stopped until clicked. Rejected: the
  rules would differ between the sidebar and the Layout menu for the same
  outcome.

### 9. Focus and dimming

- A pane's mouse-down focuses it before the event reaches the content, so
  one click both focuses the pane and lands in the terminal or the
  composer.
- The focused pane's input or terminal takes the keyboard focus handle.
- Unfocused panes get a black overlay at 0.1 opacity that does not take
  events. The value is the Swift app's (`Theme.swift`:15), kept as
  `UNFOCUSED_PANE_OVERLAY_OPACITY` in `consts.rs`.
- The per-pane name header is shown only in multi-pane layouts. The window's
  64 px content header keeps showing the focused agent, and holds the
  Layout menu.

## Risks / Trade-offs

- **[A mode change starts agents]** Filling panes activates up to three
  agents (decision 8). → The Layout menu only fills with agents of the same
  workspace, and the user chose the mode. It's documented in the user docs.
- **[Drag hold looks rough on long drags]** Blank or clipped edges show
  while dragging, which the Swift app never did. → It is brief, it is the
  price of one PTY resize per drag, and a gpui-terminal pause can replace it
  later without changing behaviour (decision 6).
- **[Per-frame resize on window resize remains]** A window resize still
  refits on every frame, as today. → Out of scope; the spec only covers
  divider drags.
- **[`selected_agent` drift]** A second writer could reintroduce drift. →
  One writer function, plus a `debug_assert!` in the layout render that
  `selected_agent` equals the focused pane's agent.
- **[render/mod.rs at 655 lines]** → All new rendering goes in new files.
  `make size-check` is in every task's gate.
- **[Persisted data from the Swift app]** It may hold a grid with three ids,
  or ids from another workspace. → `normalize` on load (spec, "The saved
  layout is restored").

## Migration Plan

No data migration. Existing `layout_mode` strings are the enum's serde
names. Every record written by the port so far holds `single`, which
restores as today. Rollback means reverting the change: the older build
ignores the fields it never read.

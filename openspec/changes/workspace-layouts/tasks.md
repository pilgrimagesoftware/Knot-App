# Tasks

## 1. Layout mode type (`knot-core`)

- [ ] 1.1 `LayoutMode` enum with the Swift serde names and `pane_count()`. `WorkspaceUiState.layout_mode` uses it, and an unknown value reads as `Single`. Verify: records tests round-trip each mode, an unknown string loads as `Single`, and `tests/settings.rs` still reads the legacy fixture's `splitVertical`.
- [ ] 1.2 Replace the `"grid"` value in `records/tests.rs` with a real mode and retire `WORKSPACE_LAYOUT_DEFAULT`'s string form. Verify: `cargo test -p knot-core` passes.

## 2. Layout rules (`knot-agents`)

- [ ] 2.1 `store/layout.rs`: `select`, `set_mode`, `add_companion`, `remove` and `normalize` as pure functions over the UI state, the agent order and companions. Verify: one unit test per `workspace-layouts` scenario in "Changing the layout mode", "Selecting an agent from the sidebar", "Companion layout", "A new companion opens beside its creator", "A companion whose owner is not shown", "Moving an agent to another workspace", "Removal collapses the layout" and "The saved layout is restored".
- [ ] 2.2 Wire `AgentStore::remove`, `move_to_workspace` and `create_shell_companion` through those rules, and drop the `UNWIRED(#321)` markers on the layout setters. Verify: store tests check that removing a pane's agent collapses the mode, that a move leaves the target layout alone, and that a companion opens beside its owner.
- [ ] 2.3 Normalize every workspace's UI state when the store loads it. Verify: a test loads a grid with a missing agent and gets three-pane, and a test with a stale focused index gets it clamped.

## 3. Geometry and divider (`knot`)

- [ ] 3.1 `render/layout.rs`: `pane_rects` and `divider_rects` for all five modes. The three-pane secondary divider spans the right column only, and the grid's dividers run full length. Verify: unit tests for each mode at ratios 0.5 and 0.3/0.7, and clamping at 0.1 and 0.9.
- [ ] 3.2 `render/divider.rs`: 12 px hit area, 1 px line and a resize cursor for each direction. Window-level drag state with the clamped ratio set live, and persistence only on release. Verify: a gpui test drags a split divider past 25% and asserts the ratio is 0.25 and the UI-state document was written once, after release.

## 4. Panes in the workspace window

- [ ] 4.1 Factor the stopped, panel and terminal branch of `render/content.rs` into `render_agent_pane(id)`, and render the layout through it. Single mode must render exactly today's pane. Verify: the existing workspace-window tests pass unchanged, and `make size-check` passes.
- [ ] 4.2 Per-pane name header in multi-pane layouts, a dimmed overlay on unfocused panes, and click-to-focus setting `selected_agent` through the one writer. Verify: a gpui test clicks pane 1 of a split and asserts `selected_agent`, the sidebar highlight and keyboard focus moved.
- [ ] 4.3 Route every `select_agent` path (sidebar click with and without ⌘, number shortcuts, Select Agent menu) through `layout::select`, applying R1–R4. Verify: gpui tests for replacing the focused pane, focusing a shown agent or companion, ⌘-click falling to R3, and ⌘N on an agent already shown.
- [ ] 4.5 The sidebar highlights only the focused agent's own row, including a focused companion's row. Verify: a gpui test focuses a companion's pane and asserts that its row is highlighted and the owner's is not.
- [ ] 4.4 Side panels follow the focused agent across the whole layout area. Verify: a gpui test opens the git panel, focuses the other pane, and asserts the panel shows that agent's folder.

## 5. Layout menu

- [ ] 5.1 `render/layout_menu.rs` in the content header: shown with two or more agents in the agent view, Three Panes from three agents, Four Panes from four, current mode checked, and catalogue keys for every label. Verify: gpui tests for one, two and four agents, plus the catalogue-key test.
- [ ] 5.2 Choosing a mode calls `layout::set_mode` and activates the agents placed in new panes. Verify: a gpui test goes from single to split and back, asserting the agents and the focused agent.

## 6. Terminal resize on settle

- [ ] 6.1 Hold each terminal pane's inner size at its pre-drag pixel size during a drag, and lift the hold on release. Verify: a gpui test with two terminal panes drags across 100 px and asserts each terminal's grid size is unchanged mid-drag and changed exactly once after release.
- [ ] 6.2 Conversation panels are not held. Verify: a gpui test asserts a panel pane's bounds track the divider mid-drag.

## 7. Companions and removal in the window

- [ ] 7.1 New Shell Companion, New Companion… and the MCP create path show the companion beside a shown owner, with the companion focused. A user-created companion of an owner that isn't shown gives an owner+companion split, and an MCP-created one leaves the layout alone. Verify: gpui tests from single, from a split, from a split not showing the owner (both origins), matching `agent-list-ui`'s "Create a companion".
- [ ] 7.2 Removing an agent, and a shell companion's process exiting, collapse the window's layout and leave `selected_agent` on a shown agent. Verify: a gpui test removes the focused pane's agent from a three-pane layout.

## 8. Docs and gate

- [ ] 8.1 `docs/site/latest/`: a layouts section covering the Layout menu, dividers, focus, sidebar clicks including ⌘-click, companions, and the fact that showing an agent starts it. Verify: the page builds with the site's doc check, and the source contracts list `workspace-layouts`.
- [ ] 8.2 Full gate. Verify: `make fmt-check size-check declared-check lint test` exits 0, and `openspec validate workspace-layouts --strict` passes.

## Workflow follow-up

- User approval of this proposal before any task above starts.
- After merge: archive `workspace-layouts`, then open a follow-up issue to replace `selected_agent` with a computed accessor (design decision 3).

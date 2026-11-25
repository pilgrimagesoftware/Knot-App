# Tasks

## 1. State

- [x] 1.1 `PanelState::collapse_all_tool_calls`: clear the opened runs, record a collapsed override for every card drawn open, nested cards included; messages untouched. Verify: `panel_state/tests/collapse.rs` collapses running, failed, user-opened and nested cards and an opened run, and checks that messages are unchanged and later calls follow their default. A running call it closed reopens when it fails; one the user closed by hand stays closed.

## 2. Action, key and menu

- [x] 2.1 `CollapseAllToolCalls` action, fixed `secondary-alt-c` binding, View menu item after Artifacts, and the window handler registered while a conversation is on screen. Verify: `workspace_window/panel/collapse_tests.rs` presses the key in a real window and every card collapses (pressing `ctrl-alt-c` on macOS does not; removing the handler's collapse fails it); the menu item is enabled; `tests/view_menu.rs`, `tests/menu_key_equivalents.rs` and the catalogue test cover the item, key and label.

## 3. Docs and gate

- [x] 3.1 User docs: the View menu list and the conversation panel section.
- [x] 3.2 `make fmt-check size-check declared-check lint test` and `openspec validate collapse-all-tool-calls --strict` pass.

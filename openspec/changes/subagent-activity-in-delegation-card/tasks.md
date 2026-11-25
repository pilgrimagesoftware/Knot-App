# Tasks

## 1. Protocol

- [x] 1.1 `SessionUpdate::TextDelta` carries `_meta`. Verify: `knot-acp` protocol test with the adapter's stamped chunk shape.

## 2. Panel state

- [x] 2.1 Route by `claudeCode.parentToolUseId` in `fold`: nest under the delegation card, hold until it arrives, release at turn or session end. Card lookups by id reach nested cards. Verify: `panel_state/tests/subagents.rs` replays the reported interleaving (mid-word) and asserts one main segment with the table intact and the subagent items nested in order. With routing disabled, the segment and nesting tests fail.

## 3. Panel view

- [x] 3.1 Delegation card count line and nested activity while open; nested cards keep their own toggle. Verify: `panel_view/tests/subagent_activity.rs`, and `a_nested_card_toggles`.

## 4. Gate

- [x] 4.1 `make fmt-check size-check declared-check lint test` and `openspec validate subagent-activity-in-delegation-card --strict` pass.

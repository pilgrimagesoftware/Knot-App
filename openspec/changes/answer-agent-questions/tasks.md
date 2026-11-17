# Tasks

## 1. ACP client (`knot-acp`)

- [ ] 1.1 `InitializeParams` sends `clientCapabilities` with `elicitation.form = {}` (no `url`); test the serialized request
- [ ] 1.2 `protocol/elicitation.rs`: `QuestionRequest`, `FormField`, `FieldKind`, `Choice`; parse `properties` in wire order through a visitor (no `preserve_order`); fixture from the probe's AskUserQuestion request, an MCP-style form with every field type, a 12-field order test; URL mode, nested object and unknown types rejected
- [ ] 1.3 `QuestionAnswer::{Submitted, Skipped, Cancelled}` to `accept`/`decline`/`cancel` with typed values; empty optional fields omitted
- [ ] 1.4 Dispatch `elicitation/create`: refuse with `-32602` when unparseable; otherwise one pending map for both kinds, `SessionEvent::Question`, resolved event generalised to `InputResolved`
- [ ] 1.5 Stop and session end answer pending questions `cancel`; permissions unchanged
- [ ] 1.6 `$/cancel_request`: answer `-32800`, drop the pending entry, emit `InputResolved`, ignore unknown ids; tests for a question, a permission, and a late caller answer

## 2. Panel state and gating (`knot`)

- [ ] 2.1 `PendingInput::{Permission, Question}` queue in `PanelState` replacing `pending_permission` + `queued_permissions`; arrival order across kinds; fold the new events; `subagent_feed` ignores them
- [ ] 2.2 Gating sites read `pending_input`: `pane.rs`, `prompt.rs` (send and queued delivery), `steer.rs`, `panel_activity.rs` (Awaiting input with the first question as detail), `idle.rs`; tests per site with a question pending
- [ ] 2.3 `SkipReason::QuestionPending` in `knot-core` with its catalog key; `scheduling.rs` returns it; test
- [ ] 2.4 Permission-only actions match `PendingInput::Permission` only; test that allow/deny do nothing with a question shown

## 3. Question card

- [ ] 3.1 `QuestionState`: picks, toggles, Other text, number validation, required check, Submit availability; pure tests
- [ ] 3.2 Render the card at the permission row's position: fields, choice descriptions, `<key>_custom` as that question's Other, Submit and Skip; catalog keys for every string
- [ ] 3.3 Keyboard: focus on show, Up/Down, Tab/Shift-Tab, Space, Return (to next question past Other), Cmd-Return, Escape; navigation as a pure function over the field list with tests; actions in the fixed keymap with hints on the buttons; focus back to the composer on removal
- [ ] 3.4 Live check against `claude-agent-acp`: AskUserQuestion with a single and a multiple question answered, skipped, and stopped

## 4. Docs and gate

- [ ] 4.1 `docs/site/latest/agents.md`: answering an agent's question; `reference/keybindings.md`: the card's keys
- [ ] 4.2 `make` passes clean

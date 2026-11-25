# Tasks

## 1. Model

- [ ] 1.1 Give `AgentMenuEntry` an optional Shift alternate:
  - Restart Agent → Restart with New Conversation.
  - Deactivate → Deactivate with New Session, a new variant with its label
    key and its menu bar action.
- [ ] 1.2 `agent_context_menu_entries`: the context menu folds each pair into
  one line. The alternate is offered only for a non-companion, non-shell
  agent. The Agents menu keeps both, the alternate directly below its
  primary.
- [ ] 1.3 Unit tests:
  - the folded list
  - the menu bar list
  - no alternate for a shell or a companion
  - every alternate label resolves

## 2. Deactivate with New Session

- [ ] 2.1 Add `AgentStore::deactivate_fresh`: clear the session ids
  `restart` clears, then run `deactivate`'s cascade. Store tests:
  - the ids are cleared
  - companions are deactivated
  - the agent is not started
- [ ] 2.2 Confirmation with its own catalogue keys, then deactivate and
  persist. Window test: confirm, then select, and the agent starts a new
  session.

## 3. Context menu rendering

- [ ] 3.1 Build an alternate line as `PopupMenuItem::element`:
  - The title is chosen from `window.modifiers().shift`.
  - A muted ⇧ marker sits in the trailing column.
  - The handler reads `window.modifiers()`, not the `ClickEvent`.
- [ ] 3.2 In the root `on_modifiers_changed` listener, call
  `window.refresh()` when Shift changes. The `key_hints` call stays as it
  is.
- [ ] 3.3 Bind `shift-enter` to `Confirm` in the `PopupMenu` context.
- [ ] 3.4 gpui tests. Each must fail with its fix reverted:
  - an open menu's title swaps on a simulated Shift change
  - a Shift-click runs the alternate
  - Shift-Return runs the alternate
  - a shell's line does not swap

## 4. Specs, docs, gate

- [ ] 4.1 Update `docs/site/latest` (agent menu): describe the ⇧ marker and
  the alternates.
- [ ] 4.2 `openspec validate menu-modifier-alternates --strict`.
- [ ] 4.3 Run `make fmt-check size-check declared-check lint test`. CI must
  pass on ubuntu, macOS and windows.
- [ ] 4.4 Follow-up issue: native NSMenu alternates in gpui (design,
  option B).

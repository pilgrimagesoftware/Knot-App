# Tasks

Sections 1-5 ship in #506. Section 6 ships in a stacked draft PR.

## 1. Model

- [ ] 1.1 `AgentMenuEntry`: add Deactivate with New Session (label key, menu
  bar action), and `shift_alternate(facts)`: Restart Agent → Restart with
  New Conversation, Deactivate → Deactivate with New Session, offered only
  for a non-companion, non-shell agent with restored conversations on
  (`AgentMenuFacts::restores_conversations`).
- [ ] 1.2 `agent_context_menu_entries` lists primaries only; the menu bar's
  list (option A until section 6) puts each alternate directly below its
  primary.
- [ ] 1.3 Sidebar: Restart All → Restart All with New Conversations,
  Deactivate All → Deactivate All with New Sessions, under the same rule,
  disabled with their primary.
- [ ] 1.4 Unit tests: the folded list, the menu bar list, no alternate for a
  shell, a companion or with the setting off, the sidebar alternates, every
  new label resolves.

## 2. Store

- [x] 2.1 `AgentStore::deactivate_fresh`: clear what `restart` clears
  (including `fresh_context_requested`), replace `restart_token`, then run
  `deactivate`'s cascade. Tests: ids cleared, token replaced, companions
  deactivated, agent not started.
- [x] 2.2 `AgentStore::set_acp_session_id_if_current(id, token, session_id)`.
  Tests: written with the same token; dropped after `restart` and after
  `deactivate_fresh`; written after plain `deactivate`.
- [x] 2.3 `panel/session.rs`: capture the token at launch and write through
  the guard.

## 3. Window actions and confirmations

- [ ] 3.1 Deactivate with New Session: confirm, then `deactivate_fresh`,
  tear the sessions down, persist.
- [ ] 3.2 Restart All with New Conversations and Deactivate All with New
  Sessions: confirm with their own keys, then run per agent.
- [ ] 3.3 Window test: Deactivate with New Session, then select: the agent
  starts without a session to load.

## 4. Context menu rendering

- [ ] 4.1 An alternate line as `PopupMenuItem::element`: title from
  `window.modifiers().shift`, a muted ⇧ in the trailing column, handler
  reading `window.modifiers()`. Shared by the row and sidebar menus.
- [ ] 4.2 Root `on_modifiers_changed`: `window.refresh()` when Shift changes.
- [ ] 4.3 Bind `shift-enter` to `Confirm` in the `PopupMenu` context.
- [ ] 4.4 gpui tests, each failing with its fix reverted: the title swaps on a
  simulated Shift change; a Shift-click runs the alternate; Shift-Return
  runs the alternate; a shell's line does not swap; no ⇧ with the setting
  off.

## 5. Docs and gate (#506)

- [ ] 5.1 `docs/site/latest`: the ⇧ marker and the alternates in the agent
  and sidebar menus.
- [ ] 5.2 `openspec validate menu-modifier-alternates --strict`.
- [ ] 5.3 `make fmt-check size-check declared-check lint test`; CI green on
  ubuntu, macOS and windows.

## 6. Native menu-bar alternates (stacked PR)

- [ ] 6.1 Create `pilgrimagesoftware/gpui-pre`: import the published
  `gpui-pre` and `gpui-pre-macos` 0.3.7 sources, then branch
  `knot/menu-alternates-0.3.7`.
- [ ] 6.2 Patch: an alternate marker on `MenuItem::Action` with a builder;
  `setAlternate:` and `setKeyEquivalentModifierMask:` in gpui-pre-macos.
- [ ] 6.3 `[patch.crates-io]` beside `gpui-base`, with a comment; ADR
  `docs/adr/0009-gpui-pre-menu-alternates.md` (commits, exit plan, drafted
  upstream PR text).
- [ ] 6.4 The Agents menu emits each alternate as a native alternate of its
  primary on macOS and the primary only elsewhere; tests for both shapes.
- [ ] 6.5 Docs, gate, CI green on all jobs.

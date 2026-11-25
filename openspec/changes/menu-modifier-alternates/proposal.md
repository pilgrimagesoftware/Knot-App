# Proposal

## Why

Some agent actions are variants of another action: "Restart with New
Conversation" is Restart Agent with one decision changed. Adding a menu line
for each variant makes the context menu longer every time (#475). macOS
handles this with alternate items. Hold a modifier while a menu is open, and
an item's title changes to its variant: Save to Save All, or Print… to
Print. The variant is never hidden, because the title on screen always names
the action that a click will run.

The issue asks for this behaviour with Shift. It explicitly rules out a
Shift-click that changes what happens without changing what is shown.

## What Changes

- **Agent row context menu:**
  - An item with a Shift variant shows its primary title. While Shift is
    held, the open menu redraws the title as the variant's title.
  - Choosing the item, by click or by Enter, runs whichever action is
    showing.
  - Each such item also shows a muted ⇧ in its trailing column. This makes
    the variant discoverable before the user tries Shift.
- **Restart pair:** Restart Agent and Restart with New Conversation merge
  into one line. Restart Agent is the primary title, and Restart with New
  Conversation is its Shift variant. Each keeps its own confirmation, and
  Restart with New Conversation keeps its visibility rule: an owner that is
  not a shell.
- **New Deactivate variant:** Deactivate gains a Shift variant, "Deactivate
  with New Session". It deactivates the agent as Deactivate does, and also
  clears the agent's session ids, so its next start begins a new
  conversation instead of resuming. Because it discards the resume point,
  it asks for confirmation first. A shell has no conversation to discard,
  so it gets no variant.
- **Agents menu bar:** the bar keeps listing both items of each pair. The
  native menu cannot show alternates yet (see design), so it gains one line,
  Deactivate with New Session, for the new variant.
- **Spec drift:** the agent row context menu's item list in `agent-list-ui`
  is brought up to date. It predates Agent Info, Config Files, Lock, Unlock,
  the Autopilot items and Deactivate, which the menu already shows.

Non-goals:

- Alternates in the sidebar background menu (Restart All, Deactivate All).
  This is left for a follow-up once the agent row pattern has been used for
  a while; see design, open questions.
- Pairing Fork Agent with Duplicate Agent, Save to Bench with Bench Agent…,
  or New Companion… with New Shell Companion. Each pair differs in more than
  one decision, has its own shortcut, or hides a destructive action behind a
  modifier. They stay as separate lines.
- Native NSMenu alternates in the menu bar. They need a change to gpui; see
  design, option B.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-list-ui`:
  - The agent row context menu's item list.
  - Its visibility rules.
  - A new requirement for Shift alternates.
- `agent-lifecycle`: a new requirement, Deactivate with New
  Session.
- `app-menu`: the Agents menu lists alternates as separate items, which is
  where the menu bar now differs from the context menu's layout.

## Impact

- `crates/knot/src/agent_menu_entries.rs`:
  - Each entry gains an optional Shift alternate.
  - The context menu builder folds a pair into one line.
- `crates/knot/src/workspace_window/menus/agent_row.rs`: builds the
  alternate item, which renders from `window.modifiers()` and runs from it.
- `crates/knot/src/workspace_window/render/root.rs`: the root
  `on_modifiers_changed` listener refreshes the window when Shift changes.
- PopupMenu keymap: `shift-enter` is bound to `Confirm` in the PopupMenu
  context, so the keyboard reaches the variant.
- `crates/knot-agents/src/store/lifecycle.rs`: deactivate with fresh
  session ids.
- `crates/knot/src/agent_menu.rs`: a new menu bar action for Deactivate with
  New Session.
- Localization keys:
  - `menu.agent.deactivate_new_session`
  - the ⇧ marker's tooltip, if one is kept
- Docs: the agent menu page.
- No new dependencies and no `[patch]` changes.

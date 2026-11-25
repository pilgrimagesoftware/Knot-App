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
  - Choosing the item, by click, Return or Shift-Return, runs whichever
    action is showing.
  - Each such item also shows a muted ⇧ in its trailing column, on every
    platform. This makes the variant discoverable before the user tries
    Shift.
  - A variant (and its ⇧) is offered only when it differs from its primary:
    for an owner that is not a shell, with `restore-conversation-on-launch`
    on. With the setting off, Restart already starts a new conversation.
- **Restart pair:** Restart Agent and Restart with New Conversation merge
  into one line. Restart Agent is the primary title, and Restart with New
  Conversation is its Shift variant. Each keeps its own confirmation.
- **New Deactivate variant:** Deactivate gains a Shift variant, "Deactivate
  with New Session". It deactivates the agent as Deactivate does, and also
  clears the agent's session ids, so its next start begins a new
  conversation instead of resuming. Because it discards the resume point,
  it asks for confirmation first.
- **Sidebar background menu:** Restart All and Deactivate All gain the same
  kind of Shift variant: Restart All with New Conversations and Deactivate
  All with New Sessions, under the same rule and marker.
- **Session-id race:** a session id reported by a connection that started
  before a restart or a Deactivate with New Session is no longer written
  back, so "new session" holds.
- **Agents menu bar:** on macOS, each pair folds into one native alternate
  item (Restart Agent ⌘R / Restart with New Conversation ⇧⌘R). This needs
  a small gpui-pre patch, carried as a fork under `[patch.crates-io]` with an
  ADR. Windows and Linux show the primary only. Until the fork lands, each
  variant is its own item directly below its primary.
- **Spec drift:** the agent row context menu's item list in `agent-list-ui`
  is brought up to date. It predates Agent Info, Config Files, Lock, Unlock,
  the Autopilot items and Deactivate, which the menu already shows.

Non-goals:

- Pairing Fork Agent with Duplicate Agent, Save to Bench with Bench Agent…,
  or New Companion… with New Shell Companion. Each pair differs in more than
  one decision, has its own shortcut, or hides a destructive action behind a
  modifier. They stay as separate lines.
- Opening the gpui upstream PR. Its text is drafted in the ADR.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-list-ui`:
  - The agent row context menu's item list.
  - Its visibility rules.
  - New requirements for Shift alternates and the ⇧ marker.
  - Restart All and Deactivate All gain Shift alternates.
- `agent-lifecycle`: a new requirement, Deactivate with New Session, and a
  guard on late session ids.
- `app-menu`: the Agents menu folds each pair into a native alternate on
  macOS, and shows the primary only elsewhere.

## Impact

- `crates/knot/src/agent_menu_entries.rs`: each entry gains an optional Shift
  alternate, offered by the agent's facts.
- `crates/knot/src/app_state.rs`: the sidebar background entries gain theirs.
- `crates/knot/src/workspace_window/menus/`: the alternate line (render from
  `window.modifiers()`, run from it) for both menus.
- `crates/knot/src/workspace_window/render/root.rs`: the root
  `on_modifiers_changed` listener refreshes the window when Shift changes.
- PopupMenu keymap: `shift-enter` is bound to `Confirm` in the PopupMenu
  context.
- `crates/knot-agents/src/store/`: `deactivate_fresh` and
  `set_acp_session_id_if_current`.
- `crates/knot/src/workspace_window/panel/session.rs`: the guarded write.
- `crates/knot/src/agent_menu.rs`: menu bar actions for the new variants, and
  the native alternate items.
- `Cargo.toml`: `[patch.crates-io]` for `gpui-pre` and `gpui-pre-macos`
  (stacked PR), and `docs/adr/0009-gpui-pre-menu-alternates.md`.
- Localization keys for the new titles and confirmations.
- Docs: the agent menu and sidebar pages.

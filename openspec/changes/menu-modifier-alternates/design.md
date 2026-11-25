# Design

## Spike findings

These were checked against the pinned versions in `Cargo.lock`: gpui-pre
0.3.7, gpui-component 0.7.0 and gpui-kit 0.7.0. A throwaway gpui test
drove both the Layout dropdown and the agent row's right-click context
menu. The test was not committed.

### 1. An open popup does not redraw on a modifier change by itself

- `PopupMenu` (gpui-component) registers no modifiers listener.
- gpui writes the new state to `window.modifiers` and dispatches
  `ModifiersChangedEvent`, but does not mark the window dirty.
- The test opened the menu and then simulated Shift down. Neither the
  dropdown nor the right-click context menu re-rendered (0 renders after
  Shift).

### 2. A window refresh from the root listener makes it redraw

- Modifier listeners run along the focused element's dispatch path.
  `window.rs` (`dispatch_modifiers_changed_event`) walks from the focused
  node up to the root.
- The open popup holds focus, and the popup is painted inside the workspace
  window's element tree. So the root's existing `on_modifiers_changed`
  listener (`render/root.rs`, today used by `key_hints`) does fire while a
  menu is open.
- Calling `window.refresh()` there re-renders the popup. An item built with
  `PopupMenuItem::element(|window, _| …)` reads `window.modifiers().shift`
  as `true` on that frame. This held in both menus: the dropdown rendered 3
  times, the context menu once.
- A listener on the item itself would not work, because the item is a
  descendant of the focused node, not on the path above it.

### 3. Click and keyboard read the modifier differently

- **Click:** the `on_click` handler of a popup item received a
  `ClickEvent` whose `modifiers().shift` was `false` while Shift was held
  for the click. `window.modifiers().shift` was `true`. The handler must
  read `window.modifiers()`.
- **Keyboard:**
  - `Confirm` (Enter) calls the same handler with
    `ClickEvent::default()`, so reading `window.modifiers()` covers it too.
  - gpui-component binds only `enter` to `Confirm`, so Shift+Enter matches
    nothing. A `shift-enter` binding in the `PopupMenu` context is needed.
    This is not a `cmd-` binding, so SCAR-KEYS-01 does not apply.

### 4. The native menu bar has no alternate items

- gpui-pre's `MenuItem::Action` carries a name, an action, an OS action,
  `checked` and `disabled`. It has no alternate flag and no modifier mask
  of its own.
- gpui-pre-macos (`platform.rs`, `create_menu_item`) builds each
  `NSMenuItem` from the item's key binding. It never calls `setAlternate:`.
- AppKit already supports exactly this. The Agents menu's Restart Agent
  (⌘R) and Restart with New Conversation (⇧⌘R) are a textbook alternate
  pair: same key, the second with Shift added. But nothing in gpui can
  mark the second as alternate.
- gpui-pre is a crates.io dependency with no Knot fork. The only `[patch]`
  is `gpui-base`.

## Decisions

The maintainer approved the proposal on #506 with the changes below. They
replace the open questions this section used to end with.

### Context menus: one line, live title, run what is shown

An alternate-bearing entry is built as a `PopupMenuItem::element` whose
render closure picks the title from `window.modifiers().shift`. Its click
handler picks the action the same way.

The rule that keeps the behaviour from being hidden is this: **the title on
screen is always the action that runs**. The handler and the render read
the same modifier state, so they cannot disagree.

Each alternate-bearing line also shows a muted ⇧ in the trailing column
where key hints sit, on every platform (macOS, Windows and Linux), whether
or not Shift is held. The glyph is the same everywhere: it is a marker, not
a key-hint string, so it is not localized or spelled "Shift" off macOS.

### When an alternate is offered

An alternate, and its ⇧, is offered only when it would do something the
primary does not:

- the agent is an owner (not a companion) and not a shell; and
- `restore-conversation-on-launch` is on.

With the setting off, Restart already starts a new conversation and a
deactivated agent already starts fresh, so each pair would run the same
thing twice. The line then shows only its primary, with no ⇧, and Shift
changes nothing. The same rule applies to the sidebar's Restart All and
Deactivate All.

This drops today's separate Restart with New Conversation line when the
setting is off. Nothing is lost: with it off, Restart Agent is that action.

### Redraw only on a Shift change

The root listener calls `window.refresh()` only when `shift` differs from
the previous event, not on every modifier change. Typing a capital letter
in the composer then costs two extra frames, one per Shift edge, which is
in the same range as the keystroke's own redraw. The listener keeps its
`key_hints` call unchanged.

Considered and rejected: tracking whether an alternate-bearing menu is open
so that refreshes only happen then. That would add state on
`WorkspaceWindow` whose clearing depends on the popup's dismiss path, which
the window does not own. The cost it would save is small.

### Shift-Return in every popup

`shift-enter` is bound to `Confirm` in the `PopupMenu` context, which every
popup shares: gpui-component gives a menu no narrower context to bind in.
In a menu with no alternates (the Layout dropdown, the composer's menus)
Shift-Return therefore confirms the highlighted item exactly as Return
does. That is harmless, and it is what a user holding Shift from a previous
keystroke would expect, so the binding is not narrowed further.

### Sidebar: Restart All and Deactivate All get alternates

The sidebar background menu uses the same line, marker and handler as the
row menu:

- Restart All, Shift: Restart All with New Conversations. It confirms with
  its own wording and restarts every agent into a new conversation.
- Deactivate All, Shift: Deactivate All with New Sessions. It confirms
  (unlike Deactivate All) and runs Deactivate with New Session on every
  running agent.

Each is offered under the rule above. The background menu still shows every
item and disables what does not apply, so an alternate whose primary is
disabled is disabled too.

### Deactivate with New Session

- **Store:** `AgentStore::deactivate_fresh(id)`. It clears every field
  `restart` clears (`session_id`, `acp_session_id`, `resume_session_id`,
  `fork_session` and `fresh_context_requested`, so the two stay
  deliberately aligned), gives the agent a new `restart_token`, then runs
  `deactivate`'s cascade. It does not create a terminal: unlike restart, the
  agent stays stopped.
- **Companions:** deactivated as Deactivate does. Only the owner's session
  is cleared, since companions are shells and have none.
- **Confirmation, unlike Deactivate.** Deactivate does not confirm, because
  selecting the row brings everything back. This variant discards the
  resume point, the same loss Restart with New Conversation confirms for,
  so it confirms too, under its own keys.

### A late session id must not undo "new session"

`panel/session.rs` writes the ACP session id from the connect callback,
which can land after the user restarted or deactivated the agent with a new
session. Without a guard the cleared id is written back and the next start
resumes it. Restart with New Conversation has had this race all along;
"the next start is a new conversation" makes it a requirement.

The launch captures the agent's `restart_token` when it spawns the
connection, and the callback writes through
`AgentStore::set_acp_session_id_if_current(id, token, session_id)`, which
writes only while the token is unchanged. `restart` already replaces the
token, and `deactivate_fresh` now does too. Plain Deactivate does not: an id
that arrives after a plain Deactivate is the resume point the user kept, so
it is still written. `restart_token` had no reader outside tests, so giving
it this meaning costs nothing elsewhere.

### The menu bar: native alternates (option B), in a stacked PR

The Agents menu folds each pair natively on macOS: Restart Agent (⌘R)
becomes Restart with New Conversation (⇧⌘R) while Shift is held, and
Deactivate becomes Deactivate with New Session.

- **Fork.** gpui-pre has no source repository of its own: crates.io's
  gpui-pre 0.3.7 is a published snapshot of `zed-industries/zed` at
  `f126950` (its description says so, and its `repository` points at all of
  Zed), so there is no 0.3.7 tag to branch from. The fork is
  `pilgrimagesoftware/gpui-pre`, whose first commit imports the published
  0.3.7 sources of `gpui-pre` and `gpui-pre-macos` verbatim, and whose
  branch `knot/menu-alternates-0.3.7` carries the patch on top. Other
  projects carry gpui-pre the same way.
- **Patch.** `MenuItem::Action` gains an alternate marker (the modifiers
  that reveal it), with a builder `MenuItem::alternate(modifiers)`.
  gpui-pre-macos calls `setAlternate:` and sets
  `setKeyEquivalentModifierMask:` from it. Minimal and upstreamable; other
  platforms ignore it.
- **Wiring.** `[patch.crates-io]` in `Cargo.toml`, beside the `gpui-base`
  patch, with a comment, and ADR `docs/adr/0009-gpui-pre-menu-alternates.md`
  modelled on 0003: the commits carried, and an exit plan of an upstream
  PR. That PR is drafted in the ADR, not opened.
- **Windows and Linux.** Their menu bars have no alternates, so they show the
  primary item only. The Shift variant stays reachable from the context
  menu, and Restart with New Conversation through its own key binding.
- **Keystrokes.** The native pair is macOS-only, so its `cmd-` bindings stay
  as they are (SCAR-KEYS-01 allows macOS-only actions); nothing new is bound
  with `cmd-`.

Until the stacked PR lands, #506's menu bar lists each alternate as its own
item directly below its primary (the earlier option A). The stacked PR
replaces that with the native fold.

### Two PRs

- **#506:** this proposal, the context menu, the sidebar alternates,
  Deactivate with New Session and the race fix. Menu bar as option A.
- **Stacked draft PR:** the gpui-pre fork, the `[patch]`, the ADR and the
  native menu-bar alternates, so the fork is reviewed on its own.

## Inventory of natural pairs

| Primary | Shift variant | Verdict |
| --- | --- | --- |
| Restart Agent | Restart with New Conversation | Merge (exists as two lines today) |
| Deactivate | Deactivate with New Session | New variant (issue example) |
| Restart All (sidebar) | Restart All with New Conversations | New variant (maintainer) |
| Deactivate All (sidebar) | Deactivate All with New Sessions | New variant (maintainer) |
| Fork Agent | Duplicate Agent | Keep separate: different flows (fork opens the editor), own shortcuts ⌥⌘F / ⌘D |
| Save to Bench | Bench Agent… | Keep separate: the variant removes the agent, a destructive change behind a modifier |
| New Companion… | New Shell Companion | Keep separate: different creation flows, own shortcut ⇧⌘S |

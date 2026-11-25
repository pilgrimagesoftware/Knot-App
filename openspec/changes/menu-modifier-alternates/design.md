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

### Context menus: one line, live title, run what is shown

An alternate-bearing entry is built as a `PopupMenuItem::element` whose
render closure picks the title from `window.modifiers().shift`. Its click
handler picks the action the same way.

The rule that keeps the behaviour from being hidden is this: **the title on
screen is always the action that runs**. The handler and the render read
the same modifier state, so they cannot disagree.

The title swap alone is how macOS does it. It is visible while Shift is
held, but there is no hint before then. #475 explicitly rules out hidden
behaviour, so each alternate-bearing item also shows a muted ⇧ in the
trailing column where key hints sit. The ⇧ shows whether or not Shift is
held, so the user can learn which items have a variant without trying
Shift on each one. This is a proposal; see open question 1.

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

### The menu bar: option A now, option B as a follow-up

- **A (proposed):** the Agents menu keeps one item per action. It lists
  both halves of each pair, as today, plus Deactivate with New Session.
  - `app-menu` already argues that a menu bar is navigated by position, so
    the bar is the menu that tolerates the extra line.
  - The shared entry list stays the single source. Each pair is marked
    once, and the context menu folds it while the menu bar does not.
  - No gpui fork.
- **B (follow-up):** add `alternate: bool` (or a modifier mask) to gpui's
  `MenuItem::Action`, and call `setAlternate:` in gpui-pre-macos. The bar
  then folds pairs natively, with ⌘R and ⇧⌘R swapping in place.
  - This needs a fork of gpui-pre and gpui-pre-macos under `[patch]`, with
    an ADR and an exit condition, the same way the `gpui-base` fork is
    carried. Or it needs an upstream PR first.
  - It is worth doing upstream. It is not worth blocking #475 on.

### Deactivate with New Session

- **Store:** `AgentStore::deactivate_fresh(id)`. It clears the same session
  fields `restart` clears (`session_id`, `acp_session_id`,
  `resume_session_id`, `fork_session`), then runs `deactivate`'s cascade.
  It does not create a new terminal: unlike restart, the agent stays
  stopped.
- **Companions:** they are deactivated as Deactivate does. Only the owner's
  session is cleared, since companions are shells and have none.
- **Confirmation, unlike Deactivate.** Deactivate does not confirm,
  because selecting the row brings everything back. This variant discards
  the resume point, the same loss Restart with New Conversation confirms
  for, so it confirms too, under its own keys in the restart-new wording
  pattern (open question 4).

## Inventory of natural pairs

| Primary | Shift variant | Verdict |
| --- | --- | --- |
| Restart Agent | Restart with New Conversation | Merge (exists as two lines today) |
| Deactivate | Deactivate with New Session | New variant (issue example) |
| Restart All (sidebar) | Restart All with New Conversations | Possible later |
| Deactivate All (sidebar) | Deactivate All with New Sessions | Possible later |
| Fork Agent | Duplicate Agent | Keep separate: different flows (fork opens the editor), own shortcuts ⌥⌘F / ⌘D |
| Save to Bench | Bench Agent… | Keep separate: the variant removes the agent, a destructive change behind a modifier |
| New Companion… | New Shell Companion | Keep separate: different creation flows, own shortcut ⇧⌘S |

## Open questions for the maintainer

1. **The ⇧ marker:** keep it, or rely on the macOS-style title swap alone?
   This design proposes keeping it.
2. **Menu bar:** is option A acceptable for now, with B filed as an
   upstream gpui issue? Or should #475 wait for native alternates?
3. **Sidebar alternates:** should Restart All and Deactivate All get
   variants in this change, or in a follow-up?
4. **Confirmation:** should Deactivate with New Session confirm? This
   design proposes yes, because the resume point is lost.

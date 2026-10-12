# Design

## Context

- `Prompt { id, name, text }` lives in `knot-core`
  (`settings/prompts.rs`) and is persisted to `prompts.json`.
- The `/` lookup (`workspace_window/panel/lookup.rs`) lists library prompts
  through `LookupEntry::library_prompt` and inserts one with
  `expand_library_prompt` (`panel/prompt_expansion.rs`). That function reads
  the `ContextSource`, runs the blocking `read_context` on the background
  executor, and then `apply_expansion` replaces the slash-token range, but
  only if the buffer has not changed in the meantime.
- Variables live in `knot-agent-launch/src/variables.rs`: the
  `PromptVariable` enum, `ALL`, `PromptContext`, and the pure `expand`.
  `prompt_text.rs` maps each variable to its description and drives the
  editors' insert-variable menus.
- gpui gives clipboard access on the UI thread only, through
  `cx.read_from_clipboard()`. The composer already reads images that way
  (`panel/prompt.rs`). There is no other clipboard crate.
- The composer's control bar (`panel/input/controls.rs`) already has
  searchable popover selectors (`config_select.rs`, `SEARCHABLE_SELECTORS`)
  that the picker can copy.

## Goals / Non-Goals

**Goals:**

- One expansion path for both entry points, so `{{clipboard}}` and the stale
  buffer guard behave the same whichever one the user takes.
- Keep `expand` pure. The clipboard is read once, on the UI thread, and
  passed in as data.

**Non-Goals:**

- Replacing the `/` lookup's prompt entries.

## Decisions

### Clipboard is captured at the moment of insertion

`expand_library_prompt` calls `cx.read_from_clipboard()` and takes the text
entries joined by `item.text()`, or `""` when there are none. It does this
before `spawn_in`, puts the result on `ContextSource`, and `read_context`
copies it into `PromptContext`. Reading on the UI thread is required by
gpui. Reading before the async hop also pins the value the user saw when
they chose the prompt, even if the clipboard changes while the branch is
resolving.

Alternative considered: read the clipboard after resolution in
`apply_expansion`. This was rejected because it adds a second UI-thread hop
for no benefit, and it can pick up a clipboard change the user made after
choosing the prompt.

### Startup prompts get an empty clipboard

`ContextSource` for a startup prompt sets `clipboard: String::new()`. A
startup prompt fires on launch and on restart, often while the user is doing
something else. Sending whatever happens to be on the clipboard, which could
be a password or a token, to an agent nobody is looking at is a leak. The
agent editor's custom startup-prompt field, and the startup-prompt preview of
a referenced library prompt if one is shown, flag `{{clipboard}}` with a
warning that it is always empty there. The warning reuses the unknown-name
warning style and does not block save.

### Expansion targets a range, not a token

`expand_library_prompt` currently takes the slash token's range. It is
generalised to take `target: Range<usize>` plus the buffer snapshot to guard
against. The `/` lookup passes the token range. The picker passes the current
selection, which is empty at a bare caret. `apply_expansion`'s
unchanged-buffer guard is kept as it is.

### The picker is a popover in the control bar

The picker is a new `panel/input/prompt_picker.rs`, modeled on
`config_select.rs`: a button with a book icon and the tooltip "Insert Prompt
(⌥⌘I)", whose popover has a search field and the library prompts in library
order. Each row shows the name and a one-line preview, and the list filters
by substring on name or text. Up and Down, Enter and Esc behave as they do in
the `/` lookup. When the popover closes, focus returns to the composer.

The prompts are read from settings each time the popover opens, so library
edits show up without a restart.

The empty-library state shows a single row, "No saved prompts - Open
Prompts...", which opens the Library window on the Prompts tab.

The button is hidden for shell companions, which have no prompt composer.

### The shortcut is a configurable `Shortcut`

The shortcut is a new `Shortcut::InsertPrompt`, with default `alt-cmd-i`,
bound in `keymap/resolved.rs` to an `InsertPrompt` action. The panel
composer handles the action by opening its picker. When no panel composer is
focused, the action does nothing. It is a configurable shortcut, so it goes
on the Keyboard tab with the same conflict checks as the others. ⌥⌘I is not
bound anywhere in the app today. This was checked with
`grep -rn "alt-i" crates`, and the check is repeated during implementation.

## Risks / Trade-offs

- [Risk] `{{clipboard}}` can paste a large blob into the composer. →
  Mitigation: the text is inserted into an editable buffer, never sent, so
  the user sees it first. No size cap.
- [Risk] `panel/prompt.rs` is at 657 of the 700 lines the size check allows.
  → Mitigation: the picker and the clipboard read live in their own modules,
  and `prompt.rs` is not grown.

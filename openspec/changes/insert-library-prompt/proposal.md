# Proposal

## Why

Knot-App#233 asks for an easy way to put a saved prompt into an agent's
input. The panel composer can already do it, but only through the `/`
lookup. The user has to know that library prompts are listed there next to
commands and skills, and has to type a leading `/` to reach them. There is no
button and no shortcut that goes straight to the prompt library from the
composer.

Prompts that act on something the user just copied, such as "review this
diff" or "explain this stack trace", need that text pasted in by hand after
the prompt is inserted. The prompt variables cover the agent and its folder
but not the clipboard.

## What Changes

- The composer's control bar gets an Insert Prompt button. It opens a
  searchable list of the library's prompts. Choosing one inserts its text at
  the caret, with variables expanded, through the same expansion path the `/`
  lookup uses. If text is selected, the prompt replaces it. Nothing is sent.
- A new configurable shortcut, Insert Prompt (default ⌥⌘I), opens the same
  list for the focused composer. It is listed on the Keyboard tab like the
  other configurable shortcuts.
- When the library is empty, the list says so and offers to open the Prompts
  window.
- A new prompt variable, `{{clipboard}}`, expands to the system clipboard's
  text when a prompt is inserted into a composer. It is empty when the
  clipboard holds no text. Startup prompts are sent without user action, so
  there `{{clipboard}}` expands to the empty string, and the agent editor
  warns about it.
- The `/` lookup keeps listing library prompts unchanged.

## Non-Goals

- Sending a prompt directly without going through the composer.
- Inserting prompts into anything other than a panel agent's composer, such
  as a terminal agent's PTY.
- Clipboard images or files. `{{clipboard}}` is text only.
- A per-prompt shortcut, or sorting prompts by recent use.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `prompt-library`: adds `clipboard` to "Prompt variables". Adds
  requirements for the composer's Insert Prompt list, choosing from it, and
  when `{{clipboard}}` is read.
- `keybindings`: adds the Insert Prompt shortcut.

## Impact

- `crates/knot-agent-launch/src/variables.rs`: a `Clipboard` variant, an
  `ALL` entry (8 → 9), and a `clipboard` field on `PromptContext`. Tests go
  in `variables/tests.rs`.
- `crates/knot/src/workspace_window/panel/prompt_expansion.rs`: reads the
  clipboard text on the UI thread before spawning the expansion, and takes a
  target range so it can serve both the `/` token and the picker's caret or
  selection.
- `crates/knot/src/workspace_window/panel/startup.rs`: startup expansion
  passes an empty clipboard.
- `crates/knot/src/prompt_text.rs` and the l10n strings: a description for
  `clipboard`, and the startup-prompt warning.
- `crates/knot/src/workspace_window/panel/input/controls.rs`: the button and
  a new picker module beside `config_select.rs`.
- `crates/knot/src/keymap/` and `knot-core` settings: the `InsertPrompt`
  shortcut, its default and its Keyboard-tab row.
- `docs/site/latest/agents.md`: the new control, shortcut and variable.
- No saved-format change. `prompts.json` stores variables as typed.

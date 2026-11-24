# Design

## Context

Prompt text already has one placeholder syntax, the prompt variables
(`prompt-library` - Prompt variables). It is implemented in
`crates/knot-agent-launch/src/variables.rs`: a closed `PromptVariable` enum, a
pure single-pass `expand(text, &PromptContext)`, and `unknown_variables` /
`uses_variable` for the editors' warnings. Escaping (`\{{`), whitespace inside
the braces and unclosed braces are handled by two private walkers that share
those rules. `scan` drives `expand` and `uses_variable`, and `scan_names`
drives `unknown_variables`. Both need the `user_input:` change.

Library prompt text reaches a composer through exactly one function,
`WorkspaceWindow::expand_library_prompt`
(`crates/knot/src/workspace_window/panel/prompt_expansion.rs`). It has two
callers: the Insert Prompt picker (`panel/input/prompt_picker.rs:217`) and
the `/` lookup (`panel/lookup.rs:364`). It captures the clipboard and a
snapshot of the buffer when the prompt is chosen, expands on the background
executor (because `branch` runs `git`), and writes the result only if the
buffer still matches the snapshot.

Startup prompts take a separate path: `resolve_startup_prompt` + `read_context`
(`knot-agent-launch/src/startup.rs`), expanded in
`panel_session/connect.rs:254`. That path runs for fresh sessions, scheduled
runs and restarts, with no user present.

Nothing else expands library prompt text. Work-item "Send prompt to" prompts
are fixed text, MCP messages are agent-authored, and the queued-message editor
edits text that was already expanded.

## Goals / Non-Goals

**Goals**

- `{{user_input:Label}}` and `{{user_input:Label|default}}` in the existing
  syntax, with no second scanner.
- One sheet per insertion that asks for every input, used by both composer
  paths.
- No unattended path waits on a dialog.

**Non-Goals**

- Typed inputs (choice lists, numbers, files), required fields or validation.
- Remembering values between insertions.
- Inputs in persona or bench text, or in MCP-sent messages.
- Multi-line values. `{{clipboard}}` already covers pasting a block of text.

## Decisions

### Grammar lives in the existing scanner

`scan` gains a third piece, `Piece::Input { label, default }`. After the
existing trim, an inner part that starts with `user_input:` is split on the
first `|`. Both halves are trimmed. A blank label falls through to
`Piece::Text(whole)`, the same thing an unknown name does. Escaping and
unclosed braces need no new code because they are handled before the inner
part is looked at. `scan` and `scan_names` fold into one private `walk`
that emits literal text or a braced piece classified by one
`parse_inner(inner) -> Inner` (a variable, an input, a malformed input, or
unknown). `expand`, `unknown_variables`, `uses_variable`, `user_inputs` and
`malformed_inputs` all read that one walk, so nothing can disagree about
what a name is or about escaping.

A label's default is the first non-empty default among its occurrences.
"First occurrence's default" would make `{{user_input:T}} … {{user_input:T|x}}`
lose the `x` its author wrote, and an empty default (`T|`) carries no value
anyway.

Alternatives considered:

- A separate `{{? Label}}` or `${Label}` form. Rejected: it would be a second
  syntax with its own escaping, and the issue names `user_input:`.
- `=` as the default separator. Rejected: labels are often questions and
  phrases ("Branch = ?", "x=1 or 2"). `|` is rarer in prose, and Jinja/Liquid
  users read it as a filter or default.
- Duplicate labels as separate fields. Rejected: asking twice for the same
  label is never what the author meant. Reusing a label is also how a prompt
  repeats a value.

### Values travel in `PromptContext`

`PromptContext` gains `inputs: InputValues`, a newtype over
`BTreeMap<String, String>` keyed by label. `expand` looks a label up there,
falls back to the input's default, then to the empty string. This one rule
gives both behaviours:

- The composer path fills the map from the sheet. Every label is present, so
  a field the user emptied expands to empty, not to the default.
- The startup path leaves the map empty, so every input gets its default or
  nothing.

The values travel in `ContextSource`, the same way `clipboard` does:
`ContextSource` gains `inputs: InputValues`, and `read_context` moves it
into the `PromptContext` it builds. That is a struct literal
(`startup.rs:57`), so `startup.rs` changes by one field on each struct. The
startup path in `panel_session` builds its `ContextSource` with
`..Default::default()` or an explicit empty `InputValues`. The composer path
sets it from the sheet, beside the clipboard it already sets.

`expand` stays one left-to-right pass, so a value holding `{{folder}}` stays
literal by construction, as it does for variables today.

New pure functions next to `unknown_variables`:

- `user_inputs(text) -> Vec<UserInput>`: distinct labels in first-seen order,
  first default wins. The sheet is built from this.
- `malformed_inputs(text) -> bool`: true when the text has a blank-label
  input. Used by the Prompts editor's warning.
- `unknown_variables` stops reporting names that start with `user_input:`.
  A blank-label input is malformed, not unknown, so the editor gives one
  specific warning instead of a confusing unknown name.

`UserInput { label: String, default: Option<String> }` and `InputValues`
live in a new sibling module `knot-agent-launch/src/user_inputs.rs`, which
keeps `variables.rs` (220 lines) focused. They are re-exported from `lib.rs`
beside `PromptVariable`.

### The sheet is a window, opened from the one insertion path

`expand_library_prompt` splits in two:

1. On choose: read the text, the agent and the clipboard, snapshot the
   buffer, and compute `user_inputs(&text)`. If there are none, continue
   exactly as today.
2. Otherwise, open `PromptInputsSheet` and keep the captured state in its
   submit callback. Insert runs today's background expansion with the values,
   and lands through the same `apply_expansion` snapshot check.

The sheet follows `broadcast_sheet.rs`: a window of its own, because every
dialog in the app that takes more than a yes/no is a window. That also gives
"starts empty each time" for free. Window options go in
`window_options.rs` beside `broadcast_window_options`. Fields are gpui-kit
single-line inputs. Return, Tab, Cmd-Return and Esc are handled with
`on_action`, the way `broadcast_sheet` handles `Escape`.

The `WorkspaceWindow` keeps `prompt_inputs_sheets: HashMap<Uuid, WindowHandle>`
per agent composer. Choosing any prompt with inputs while that composer's
sheet is open activates the open sheet and drops the new choice. The open
sheet keeps its prompt and the values typed so far, and replacing them
silently would lose that work. The entry is removed when the sheet closes
either way.

Lifetime: the sheet holds only a `WeakEntity<WorkspaceWindow>` and the agent
id. When an agent is removed, its map entry is taken and that window is
closed. When the workspace window closes, every sheet in the map is closed.
If a sheet outlives both somehow, its Insert finds the weak entity gone and
closes without inserting anything.

Focus: Insert and Cancel both call `activate_window` on the workspace window
handle, then focus the composer's input. The focus lands after the sheet
closes, so the composer gets the key window back.

### Five-input cap

`MAX_PROMPT_INPUTS = 5` lives in `knot-agent-launch/src/user_inputs.rs`
(the crate's `consts.rs` holds launch strings, while this constant belongs
to the input model it limits). The sheet is built from
`user_inputs(text).into_iter().take(MAX_PROMPT_INPUTS)`. The left-out
inputs are simply not put into `InputValues`, so `expand`'s
value → default → empty fallback gives them the startup rule with no extra
code. The note under the fields is `t_count("prompt_inputs.more_defaulted",
n)`, with `_one` and `_other` forms. The Prompts editor warns through
`prompt_text::too_many_inputs_warning`.

A cap keeps the sheet a fixed, small size without scrolling. Inputs past the
cap degrade to the unattended behaviour instead of being refused, so an
imported prompt with more inputs still inserts.

Because the sheet is a separate window, the user can still type in the
composer while it is open. The existing snapshot rule covers this: Insert
then inserts nothing and the user's typing is kept. The alternative, a modal
overlay inside the workspace window, would block the composer but go against
the app's convention for dialogs. It would also need a focus trap that gpui-kit
does not provide.

`/` lookup: the lookup already closes when an entry is chosen, and Cancel
writes nothing, so the token stays as typed. `lookup.rs` (498 lines) and
`prompt_picker.rs` (449 lines) do not change: both already call
`expand_library_prompt` with their target range. Two limits apply. CI's
`make size-check` fails a file over 700 lines (`RUST_FILE_LINE_LIMIT`), and
CLAUDE.md's convention is 500. `lookup.rs` is two lines under the
convention, so new code goes in new files, each kept under 500 lines.

### Editors

- Prompts window editor (`library_window`): the variable list gets a final
  user-input entry that inserts `{{user_input:}}` and places the caret before
  the closing braces. A `malformed_inputs` warning is added beside the
  unknown-name warning.
- Agent editor startup field (`agent_editor/startup_prompt.rs`): one warning
  per input without a default, beside the existing `{{clipboard}}` warning.
  Neither blocks save.
- The warning text is built in `crate::prompt_text`, the module that already
  builds `unknown_variables_warning` and `startup_clipboard_warning`. It gains
  `malformed_inputs_warning` and `startup_inputs_warning`.
- The bench editor (`library_window/bench_editor.rs`) shows only the
  unknown-name warning today, not the clipboard one. This change keeps that
  parity: the bench editor gets no startup-input warning. Showing both
  startup warnings there is a separate follow-up.

### Localisation

New keys in `crates/knot-core/locales/en.yml`: the sheet's Insert and Cancel
buttons, the user-input list entry and its description, the malformed-input
warning and the startup "empty in startup prompts" warning. The sheet's title
is the prompt's name, which is user data, so it has no key. Each new key gets
an entry in `crates/knot/src/tests/l10n_catalog.rs`, because a missing key renders the key
itself.

## Risks / Trade-offs

- **Lost typing.** Editing the composer while the sheet is open drops the
  insertion when the user presses Insert. This is the existing rule for edits
  during expansion. The trade-off buys a non-modal sheet. The values the user
  typed are lost with it, which only matters for prompts with many inputs.
- **`|` in a label** cannot be written. Labels are prompts to the user, and
  none of the issue's examples need it. A future escape (`\|`) can be added
  without breaking prompts that exist.
- **Startup prompts that need an input** silently send an empty value. The
  editor warning is the guard. Blocking the launch would break scheduled
  runs, which `session-scheduling` requires to proceed unattended.
- **Text that already contains `{{user_input:...}}`** used to be left as
  typed, as an unknown name, and now prompts. No such text is expected:
  the editor flagged it as unknown until now.

## Migration Plan

None. The stored library format does not change, and prompts without inputs
behave exactly as before.

## Open Questions

None blocking. Remembering the last value per label is a natural follow-up
if users ask for it.

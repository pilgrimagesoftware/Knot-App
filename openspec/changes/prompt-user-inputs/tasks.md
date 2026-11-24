# Tasks

## 1. Grammar and expansion (`knot-agent-launch`)

- [ ] 1.1 Add `user_inputs.rs` with `UserInput { label, default }` and the
      `InputValues` newtype, re-exported from `lib.rs`; verify `make build`
- [ ] 1.2 Teach `scan` the `user_input:` piece (first `|` splits label from
      default, both trimmed, blank label left as typed) and add
      `PromptContext.inputs`, with `expand` falling back from value to
      default to empty; verify unit tests in `variables/tests.rs` for each
      `prompt-library` "User inputs" scenario (one per label, repeated
      label, inputs with variables, value not expanded twice, blank label,
      escaped input)
- [ ] 1.3 Add `user_inputs(text)` (first-seen order, first default wins) and
      `malformed_inputs(text)`, and stop `unknown_variables` reporting
      well-formed inputs; verify tests for "A user input is not unknown" and
      "A misspelled keyword is unknown"
- [ ] 1.4 Startup path: build the startup `PromptContext` with empty
      `inputs`; verify `startup/tests.rs` cases for "A startup prompt uses
      the default" and "An input without a default is empty at startup"

## 2. Inputs sheet (`knot`)

- [ ] 2.1 Add `prompt_inputs_sheet.rs` on the `broadcast_sheet.rs` pattern,
      plus `prompt_inputs_window_options` in `window_options.rs`: title from
      the prompt name, one prefilled single-line field per input, Cancel and
      Insert; verify gpui tests that open it with two inputs and assert field
      order, the prefilled defaults and first-field focus with its text
      selected
- [ ] 2.2 Keyboard handling (Tab/Shift-Tab, Return to the next field and
      Insert from the last one, Cmd-Return, Esc and window close as cancel);
      verify gpui tests for each key
- [ ] 2.3 Split `expand_library_prompt`: with no inputs, keep today's path;
      otherwise capture the clipboard and snapshot, open the sheet, and
      expand with the values on Insert through the existing
      `apply_expansion` check. Keep at most one sheet per agent composer
      (`prompt_inputs_sheets`), and bring an open one to the front. Verify
      gpui tests for "Insert substitutes the values", "Cancel inserts
      nothing", "A prompt without inputs inserts directly", "An emptied
      default expands to nothing" and "Editing the composer while the
      sheet is open"
- [ ] 2.4 `/` lookup through the same path, with no changes to `lookup.rs`;
      verify gpui tests for "Inputs are asked for before the token is
      replaced" and "Cancelling the inputs sheet keeps the token", and that
      `make size-check` still passes for `lookup.rs` and `prompt_picker.rs`
- [ ] 2.5 Add catalogue keys for the sheet's buttons in
      `knot-core/locales/en.yml`; verify `tests/l10n_catalog.rs` asserts
      they resolve

## 3. Editors

- [ ] 3.1 Add `prompt_text::malformed_inputs_warning` and
      `startup_inputs_warning` and their keys; verify unit tests in
      `prompt_text.rs` that mirror the existing clipboard-warning test
- [ ] 3.2 Prompts editor: a user-input entry at the end of the variable list
      that inserts `{{user_input:}}` with the caret before the closing
      braces, and the malformed-input warning; verify gpui tests for
      "Inserting a user input from the list" and "A blank-label input is
      flagged but saveable"
- [ ] 3.3 Agent editor startup field: show `startup_inputs_warning` beside
      the clipboard warning; verify a test for the "Startup prompt editor
      warns" (inputs) scenario, with save left enabled

## 4. Documentation

- [ ] 4.1 Document user inputs in `docs/site/latest/personas-prompts.md`
      (syntax, defaults, repeated labels, escaping, the sheet's keys, empty
      in startup prompts), with one realistic example; verify the page
      builds with the docs site's own build command

## 5. Integration

- [ ] 5.1 `make fmt-check size-check declared-check lint test` passes
- [ ] 5.2 Manual check in the running app: insert a prompt with two inputs
      from the picker and from `/`, cancel once and insert once, and start
      an agent whose startup prompt has an input with a default

## Workflow follow-up

- Change the PR from "Refs #239" to "Fixes #239" once implementation lands.
- Archive the change after review and merge.

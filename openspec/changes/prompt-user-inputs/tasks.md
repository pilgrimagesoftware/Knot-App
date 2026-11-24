# Tasks

## 1. Grammar and expansion (`knot-agent-launch`)

- [x] 1.1 Add `user_inputs.rs` with `UserInput { label, default }`, the
      `InputValues` newtype and `MAX_PROMPT_INPUTS = 5` (defined in
      `consts.rs` per the crate convention, re-exported there), re-exported from
      `lib.rs`; verify `make build`
- [x] 1.2 Fold `scan` and `scan_names` into one `walk` over a shared
      `parse_inner` (variable, input, malformed input or unknown). Add
      `PromptContext.inputs`, with `expand` falling back from value to
      default to empty. Verify unit tests in `variables/tests.rs` for each
      "User inputs" scenario (one per label, a later occurrence giving the
      default, repeated label, inputs with variables, value not expanded
      twice, blank label, escaped input)
- [x] 1.3 Add `user_inputs(text)` (first-seen order, first non-empty
      default) and `malformed_inputs(text)`, and have `unknown_variables`
      (through `scan_names`) skip well-formed inputs and report a misspelled
      keyword; verify tests for "A user input is not unknown" and "A
      misspelled keyword is unknown"
- [x] 1.4 `ContextSource.inputs` moved into `PromptContext` by
      `read_context`; the startup path passes no values. Verify
      `startup/tests.rs` cases for "A startup prompt uses the default" and
      "An input without a default is empty at startup"

## 2. Inputs sheet (`knot`)

- [x] 2.1 Add `prompt_inputs_sheet.rs` on the `broadcast_sheet.rs` pattern,
      plus `prompt_inputs_window_options` in `window_options.rs`: title from
      the prompt name, one prefilled single-line field for each of the first
      `MAX_PROMPT_INPUTS` inputs, the "N more inputs will use their
      defaults" note (`t_count`), Cancel and Insert. Verify tests for field
      order, the prefilled defaults, first-field focus with its text
      selected, and the note at 5 inputs (absent) and 6 inputs (one more)
- [x] 2.2 Keyboard handling: Tab and Shift-Tab, Return to the next field and
      Insert from the last one, Cmd-Return, and Esc or window close as
      cancel. Verify tests for each key
- [x] 2.3 Split `expand_library_prompt`: with no inputs, keep today's path.
      Otherwise capture the clipboard and snapshot, open the sheet, and on
      Insert expand with the values through the existing `apply_expansion`
      check. Verify tests for "Insert substitutes the values", "Cancel
      inserts nothing", "A prompt without inputs inserts directly", "An
      emptied default expands to nothing", "A sixth input uses its default"
      and "Editing the composer while the sheet is open"
- [x] 2.4 Activate the workspace window and focus the composer on both
      Insert and Cancel; verify a test for "Cancel returns focus to the
      composer"
- [x] 2.5 One sheet per composer (`prompt_inputs_sheets`): a second choice
      with inputs fronts the open sheet and is dropped; removing the agent
      or closing the workspace window closes its sheet. Verify tests for "A
      second choice brings the open sheet forward" and "Removing the agent
      closes its sheet"
- [x] 2.6 `/` lookup through the same path with no changes to `lookup.rs`;
      verify tests for "Inputs are asked for before the token is replaced"
      and "Cancelling the inputs sheet keeps the token"
- [x] 2.7 Catalogue keys for the sheet's buttons and note in
      `crates/knot-core/locales/en.yml`; verify
      `crates/knot/src/tests/l10n_catalog.rs` asserts they resolve, and both
      `_one`/`_other` forms exist. (Asserted in `prompt_inputs_sheet/tests.rs`
      instead: `l10n_catalog.rs` is at 688 of 700 lines.)

## 3. Editors

- [x] 3.1 Add `prompt_text::malformed_inputs_warning`,
      `too_many_inputs_warning` and `startup_inputs_warning`, with keys;
      verify unit tests in `prompt_text.rs` that mirror the clipboard-warning
      test
- [x] 3.2 Prompts editor: a user-input entry at the end of the variable list
      that inserts `{{user_input:}}` with the caret before the closing
      braces, plus the malformed-input and five-input warnings. Verify tests
      for "Inserting a user input from the list", "A blank-label input is
      flagged but saveable" and "More than five inputs are flagged but
      saveable"
- [x] 3.3 Agent editor startup field: show `startup_inputs_warning` beside
      the clipboard warning; verify a test for "Startup prompt editor warns"
      (inputs), with save left enabled. (Unit-tested in `prompt_text.rs`, as
      the clipboard warning is; the agent editor's save does not read warnings.)

## 4. Documentation

- [x] 4.1 Document user inputs in `docs/site/latest/personas-prompts.md`:
      syntax, defaults, repeated labels, the five-input limit, escaping, the
      sheet's keys, and that they are empty in startup prompts, with one
      realistic example; verify the page renders in the docs build. (Local
      Jekyll build not run: the system Ruby lacks the pinned bundler; Liquid
      `raw` tags follow the page's existing usage.)

## 5. Integration

- [x] 5.1 `make fmt-check size-check declared-check lint test` passes (CI
      limit 700 lines; new files kept under the 500-line convention)
- [ ] 5.2 (Not done by the builder: needs a human at the running app.) Manual check in the running app: insert a prompt with two inputs
      from the picker and from `/`, cancel once and insert once, and start
      an agent whose startup prompt has an input with a default

## Workflow follow-up

- Change the PR from "Refs #239" to "Fixes #239" once implementation lands.
- Archive the change after review and merge.

# Tasks

## 1. Clipboard variable

- [x] 1.1 Add `PromptVariable::Clipboard` (`clipboard`) to `crates/knot-agent-launch/src/variables.rs`: the `ALL` entry (8 → 9), the `name()` arm, a `clipboard: String` on `PromptContext`, and the `value()` arm; verify `cargo test -p knot-agent-launch` passes with new tests in `variables/tests.rs` for round-trip, expansion, and a clipboard value holding `{{branch}}` that stays literal
- [x] 1.2 Add `clipboard` to `ContextSource` in `crates/knot-agent-launch/src/startup.rs` and copy it in `read_context`; set it to `String::new()` for startup prompts in `crates/knot/src/workspace_window/panel/startup.rs`; verify a test that a startup prompt `Context: {{clipboard}}` expands to `Context: `
- [x] 1.3 Add the `description_key` arm in `crates/knot/src/prompt_text.rs` and the `prompt_variables.clipboard` l10n string; verify the insert-variable menus in the prompt editor and the agent editor list `clipboard` (`make test`)
- [x] 1.4 Add the startup-prompt warning for `{{clipboard}}` in `crates/knot/src/agent_editor/startup_prompt.rs`, which does not block save; verify with a test covering the "Startup prompt editor warns" scenario

## 2. Shared expansion with clipboard capture

- [x] 2.1 Generalise `expand_library_prompt` in `panel/prompt_expansion.rs` to take a target range, and read `cx.read_from_clipboard()` text (empty for non-text) before `spawn_in`; update the `/` lookup caller; verify the existing `prompt_expansion/tests.rs` still pass and add tests for clipboard text, image-only clipboard, and a clipboard change after choosing the prompt

## 3. Insert Prompt picker

- [ ] 3.1 Add `panel/input/prompt_picker.rs`, modeled on `config_select.rs`: the button in the control bar (hidden for shell companions), a popover with search, the filtered list, keyboard navigation, and the empty-library row that opens the Prompts window; verify `make size-check` and `make declared-check` pass
- [ ] 3.2 On choosing a prompt, expand into the composer's current selection or caret through 2.1, then refocus the composer with the caret after the insertion; verify window tests for the "Insert at the caret", "Replace a selection", "Filtering", "Empty library" and "Dismissing changes nothing" scenarios

## 4. Shortcut

- [ ] 4.1 Add `Shortcut::InsertPrompt` with default `alt-cmd-i` and an `InsertPrompt` action in `crates/knot/src/keymap/`, plus its Keyboard-tab row and l10n; first confirm with `grep -rn "alt-i\|alt-cmd-i\|cmd-alt-i" crates` that the chord is free; verify tests for the three `keybindings` scenarios, alongside `crates/knot/src/tests/settings_keyboard.rs`
- [ ] 4.2 Handle `InsertPrompt` in the focused panel composer by opening its picker, and do nothing elsewhere; verify with a window test

## 5. Integration

- [ ] 5.0 Document the Insert Prompt control, its shortcut and `{{clipboard}}` (including that it is empty in startup prompts) in `docs/site/latest/agents.md` beside the existing prompt-variable docs; verify the page lists every variable in `PromptVariable::ALL`
- [ ] 5.1 Update `CHANGELOG.md` only if the repo's release process expects it (check `cliff.toml`; skip if generated); run `make` (fmt-check, size-check, lint, test, build); verify it passes
- [ ] 5.2 Launch the app with a throwaway store, add a prompt using `{{clipboard}}`, copy text, insert the prompt with the button and with ⌥⌘I, and confirm the result; verify by recording it in the PR description

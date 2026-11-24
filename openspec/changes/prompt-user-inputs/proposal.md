# Proposal

## Why

A library prompt can adapt to the agent it is sent to (`{{branch}}`,
`{{folder.name}}`, `{{clipboard}}`), but not to the task at hand. A prompt
such as "Review PR #___ with focus on ___" has to be inserted and then
hand-edited at each blank. #239 asks for named inputs, written as
`{{user_input:What is your quest?}}`, that Knot asks for in one dialog when
the prompt is inserted into a composer.

## What Changes

- A new placeholder form in the existing `{{...}}` syntax:
  `{{user_input:Label}}`, with an optional default, `{{user_input:Label|default}}`.
  It reuses the prompt variables' braces, whitespace, escaping (`\{{`) and
  unclosed-brace rules, so prompt text still has a single syntax.
- Inserting a library prompt that has inputs into a composer (Insert Prompt
  list or `/` lookup) first opens an "inputs" sheet with one field per
  distinct label, in the order the labels first appear, each prefilled with its
  default. Insert substitutes the values; Cancel inserts nothing and leaves the
  composer as it was. A prompt with no inputs inserts directly, as it does now.
- Repeating a label asks for it once and uses the value at every occurrence.
- The sheet asks for at most five inputs. Any more expand to their defaults,
  with a note in the sheet saying so, and the Prompts editor warns about a
  prompt that has more than five.
- Values are inserted literally and never expanded again, like every other
  variable value.
- Startup prompts are sent without the user present, so they never open the
  sheet: an input there expands to its default, or to nothing when it has no
  default. The agent editor's custom startup-prompt field warns about inputs
  that have no default, without blocking save, the same way it warns about
  `{{clipboard}}`.
- The Prompts editor lists `user_input` alongside the variables, does not flag
  a well-formed input as unknown, and flags a malformed one (blank label) as a
  warning.

## Capabilities

### New Capabilities

None. Inputs belong to the prompt text model that `prompt-library` already
owns.

### Modified Capabilities

- `prompt-library`: ADDED "User inputs" (grammar, duplicates, defaults,
  literal values), "Inserting a prompt with inputs asks for them" (the sheet),
  and "Startup prompts never ask for inputs". MODIFIED "Unknown variables and
  escaping" so that a well-formed `user_input` is not unknown.
- `panel-slash-commands`: MODIFIED "Inserting a library prompt expands its
  text" so that the `/` lookup asks for inputs before it replaces the token,
  and leaves the token as typed on Cancel.
- `library-windows`: MODIFIED "Prompts window" so that the editor's variable
  list offers a `user_input` entry and flags malformed inputs.

## Impact

- `knot-agent-launch` (`variables.rs`): the scanner recognises
  `user_input:`; a new pure `user_inputs(text)` lists the distinct inputs;
  `PromptContext` carries the values keyed by label. `expand` stays one
  left-to-right pass.
- `knot` (`workspace_window/panel/prompt_expansion.rs`): the one shared
  insertion path for the picker and the `/` lookup opens the inputs sheet
  before expansion when the prompt has inputs. A new `prompt_inputs_sheet.rs`
  window follows `broadcast_sheet.rs`.
- `knot` (`agent_editor/startup_prompt.rs`, `library_window` prompt editor):
  warnings and the variable list.
- `knot-core/locales/en.yml`: the sheet's title, buttons and the two new
  warnings.
- Not changed: the stored library format. Inputs live in the prompt text, so
  existing prompts, the Knot-Library import and submission carry them as-is.
- Out of scope, because none of these expand library prompt text: work-item
  "Send prompt to" prompts (fixed text), MCP `send-message`/`broadcast-message`,
  autopilot replies, and the queued-message editor. Scheduled runs and restarts
  send the startup prompt, so they follow the startup-prompt rule above.
- Non-goals: typed inputs (numbers, choices, files), validation or required
  fields, remembering the last value used, and inputs in the bench or persona
  text.

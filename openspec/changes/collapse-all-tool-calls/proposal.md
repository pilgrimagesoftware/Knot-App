# Proposal

## Why

Each tool-call card opens and closes on its own, and in compact mode each
summary run can be opened to its cards (#488). After a long turn the
conversation can hold many opened cards and runs, with no way back to the
compact view except closing them one at a time.

## What Changes

- **View > Collapse Tool Calls** (⌥⌘C on macOS, Ctrl+Alt+C elsewhere)
  returns everything in the selected agent's conversation to its collapsed
  form. Every summary run the user opened goes back to its summary line, and
  every tool-call card drawn open collapses to its header, including running
  and failed cards (open by default) and cards nested under a delegation.
- With compact mode off, the same action collapses every open card. Runs only
  exist in compact mode, so there are none to close.
- Only the user's open/closed choices change. The conversation and its tool
  results are untouched, a card that arrives afterwards follows its own
  default, and any card or run can be opened again by hand.
- The key is fixed, like the panel's other keys. ⌥⌘C is free in Knot, in the
  text input and in the terminal; ⇧⌘C was not chosen, because off macOS it is
  Ctrl+Shift+C, a terminal's copy key.

### Non-goals

- An "expand all" action.
- Making the key configurable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `collapsed-tool-call-summary`: adds "Collapsing every tool call at once".
- `app-menu`: adds "View > Collapse Tool Calls".
- `keybindings`: adds "Collapsing the tool calls by shortcut".

## Impact

- `knot`: `PanelState::collapse_all_tool_calls`, the session handle's
  wrapper, the `CollapseToolCalls` action with its fixed binding, the View
  menu item, and the workspace window's handler, which is registered while a
  conversation is on screen, as for Jump to Bottom.
- Docs: the View menu list and the conversation panel section.

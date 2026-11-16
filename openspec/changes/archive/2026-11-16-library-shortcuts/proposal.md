# Proposal

## Why

Personas, Prompts and Bench are opened from the menu bar only: they have no
keyboard shortcut, and `app-menu` said they should have none (#223). They are
windows a user goes back to repeatedly while setting agents up.

## What Changes

- Three configurable shortcuts: Open Personas ⌥⇧⌘P, Open Prompts ⌥⇧⌘I and
  Open Bench ⌥⌘B. They are listed and rebindable in the Keyboard settings
  pane, validated like the others, and stored only when customized.
- The menu items show them as key equivalents.

## Impact

- `knot`: `Shortcut::{OpenPersonas, OpenPrompts, OpenBench}`, their chords in
  `Resolved`, bound to the existing `OpenPersonas`/`OpenPrompts`/`OpenBench`
  actions.
- `knot-core`: `KeybindingSettings::{open_personas, open_prompts,
  open_bench}`; their labels in `en.yml`.
- The bench popover's per-agent rows stay unbound: they are a list that
  changes with the user's bench, not menu items.

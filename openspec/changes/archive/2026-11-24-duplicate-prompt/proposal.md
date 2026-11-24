# Proposal

## Why

Issue #469: there is no way to duplicate a library prompt. Making a variant of
one means opening "Add Prompt…" and retyping or pasting its text.

## What Changes

- The library gains a duplicate operation: a copy with a new id, the
  original's text, description and every other field, named "`<name>` copy"
  (or "`<name>` copy 2", "copy 3"... when that name is taken), inserted right
  after the original and persisted like any new prompt.
- The Prompts window's rows get a Duplicate action beside edit and delete.
- The Insert Prompt list's rows get a duplicate button beside the pencil.
- Both open the prompt editor on the copy with the caret in its name. The copy
  is an existing prompt, so the editor saves rather than creates, and
  canceling keeps the copy as made.

## Impact

- `knot-core`: `Settings::duplicate_prompt`, `copy_name`; two catalogue keys
  for the copy's name.
- `knot`: the Prompts window row, the Insert Prompt row, and the prompt
  editor's entry point for a copy; two catalogue keys for the tooltips.
- Specs: MODIFIED `prompt-library` "Library operations" (duplicate, and the
  copy's place in the order); ADDED `prompt-library` "Duplicating a prompt
  from the Insert Prompt list"; ADDED `library-windows` "Duplicating a prompt
  from the Prompts window".
- `prompt-user-inputs` (#472, #239) MODIFIES `library-windows` "Prompts
  window". The Duplicate action is an ADDED requirement here rather than a
  second MODIFIED of that one, so the two changes archive in either order
  without one overwriting the other. Inputs live in the prompt's text, so a
  copy carries them with no further change.
- Names stay non-unique ("Prompt model" is unchanged): the copy's name only
  avoids clashing at the moment it is made.

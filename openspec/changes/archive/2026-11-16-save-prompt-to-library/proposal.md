# Proposal

## Why

A prompt that worked well in a conversation could only be copied; keeping it
for reuse meant opening the Prompts window and pasting it into a new prompt by
hand (#42).

## What Changes

- Beside a sent prompt's copy control, revealed the same way, a save control
  opens the prompt editor on a new prompt holding that prompt's text, with the
  caret in its name.

## Impact

- `knot`: the prompt bubble's hover controls; the prompt editor gains an entry
  point for a new prompt with given text (`open_prompt_editor_with_text`).

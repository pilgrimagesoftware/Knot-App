# Proposal

## Why

When an agent shows a markdown file over the one the user is reading, the
earlier file is replaced, and the only way back was the agent's context-menu
"Markdown Files" submenu - which the user, looking at the panel, did not know to
look for (#29).

## What Changes

- While the agent has shown other markdown files before the current one, the
  markdown section's header offers a control listing them, most recent first.
  Choosing one shows it again, unexpanded, as the Markdown Files menu does.

## Impact

- `knot`: a new `artifact_panel::history` module and one control in the
  markdown section's header.

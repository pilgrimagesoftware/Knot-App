# Proposal

## Why

The markdown section's Approve sent `approved let's do it`, which does not say
what was approved: whenever anything else was pending in the conversation the
agent could not tell what the user had agreed to (#30). And a user part-way
through reviewing a file could only send their comments, not approve with them.

## What Changes

- Approve sends `Approved: <file>`, naming the file relative to the agent's
  folder when it lies inside it.
- For a Panel-mode agent, Approve stays on offer during a review and sends the
  approval with the review comments the user wrote.

## Impact

- `knot`: the artifact panel's review flow and controls.

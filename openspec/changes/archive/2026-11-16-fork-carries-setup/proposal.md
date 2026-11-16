# Proposal

## Why

Fork Agent pre-fills the agent editor through `AgentPrefill`, which had no
fields for the description, capabilities, cost tier, activation mode, shell
command or session setup (model, permission mode, effort). A fork lost all of
them: the same gap Duplicate Agent had before #177 (#38).

## What Changes

- `AgentPrefill` carries the rest of the source's setup, and the editor's
  create form starts from it, so the user sees, and can change, the source's
  description, capabilities, cost tier and activation mode before creating.
- Create gives the new agent the prefill's session setup, as a duplicate gets
  its source's.
- `AgentPrefill::fork_of` builds the fork's prefill by destructuring `Agent`
  without `..`, as `AgentStore::duplicate` does, so a field added to `Agent`
  fails to compile until Fork decides what to do with it.

## Impact

- `knot`: `agent_editor::fork` (new), `AgentPrefill`, the editor's create
  defaults and submit, and the Fork Agent menu entry.
- Spec: `agent-list-ui`'s Fork Agent requirement lists the whole setup.

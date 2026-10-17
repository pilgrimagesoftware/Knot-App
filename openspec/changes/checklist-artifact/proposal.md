# Proposal

## Why

Agents regularly need the user to answer several questions or confirm several
things before they can go on. Today those lists arrive as conversation text:
they scroll away under the agent's next output, the user answers them in one
free-form reply that is easy to leave incomplete, and the agent has to parse
which answer belongs to which question (#301).

## What Changes

- A new artifact kind, the **checklist**: a title and a list of items, each a
  checkbox or a free-text question. It shows in the artifact panel beside the
  markdown and diagram artifacts.
- New MCP tool `show-checklist` (agentId, optional title, items of
  `{id, text, kind}`) that shows or updates an agent's checklist. Updating
  keeps the user's answers for items whose id and kind are unchanged.
- New MCP tool `get-checklist` (agentId) that returns the checklist as the user
  has left it: ticks, answers, and whether it was sent.
- In the panel the user ticks items and types answers, which are written to the
  agent's checklist as they happen, and presses **Send** to deliver the whole
  list to the agent as one message - a prompt for a Panel-mode agent, typed at
  the prompt for a Terminal-mode one, as the markdown section's Approve is.
- Closing the checklist remembers it, answers included, so reopening restores it.

## Impact

- `knot-agents`: `Checklist`, `ChecklistItem`, `ChecklistItemKind`; an
  `Agent::checklist` slot (runtime only); store writes.
- `knot-mcp-tools`: the two tools.
- `knot`: the artifact panel's checklist section, close/reopen, and Send.
- Specs: `mcp-tools` (catalog grows to twenty tools; two new requirements),
  `artifact-panel` (one new requirement).

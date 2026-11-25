# Design

## Context

claude-agent-acp's `stampParentToolUseId` (`dist/sdk-to-acp.js`, 0.111.0)
adds `_meta.claudeCode.parentToolUseId` to every update a subagent makes:
`tool_call`, `tool_call_update` and `agent_message_chunk`. Knot declares no
ACP subagent capability, so these share the main session stream.

## Decisions

### Route on the parent id when folding

`fold` reads the parent id from `ToolCallStart`'s and `TextDelta`'s `_meta`.
With a parent, the item goes to that card's `subagent` list; without one, it
goes where it always did. `tool_call_update` needs no routing: card lookup by
id searches nested cards, and the adapter may leave the parent id off a later
update anyway.

### Hold, then adopt or release

A parent that hasn't arrived yet is held in order while the turn runs, and
adopted when its card is pushed. On turn end or session end, anything held is
placed in the conversation. That way an update is never lost, even if the
adapter's ordering changes.

### Render inside the existing card

The tool-call card takes a collapsed-state function instead of a `bool`, so a
nested card keeps its own open/closed state through the same
`toggle_tool_call` path. The count line reuses the compact run summary's
wording through `ToolRunSummary::of_cards`.

## Risks / Trade-offs

- **[Vendor key]** The routing reads a claude-agent-acp key. Another adapter
  that doesn't send it behaves as before. #15 replaces this path.

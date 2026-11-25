# Proposal

## Why

A response's text was cut apart by tool calls it never made (#476). The
reporter's session shows one continuous reply from the main agent, written
while two background subagents it had launched ran shell commands. The
adapter (claude-agent-acp 0.111.0) forwards a subagent's tool calls and text
into the same session stream, stamped
`_meta.claudeCode.parentToolUseId`. Knot folded each one in as a top-level
entry, so the reply split wherever one landed: mid-word, and through a Markdown
table, which then failed to render.

## What Changes

- A subagent's tool calls and prose are kept inside the card of the `Agent` /
  `Task` call that launched it, never in the conversation. The main agent's
  reply stays one piece.
- The delegation card shows a count line of what its subagent ran ("5 tool
  calls · 3 commands run"), and lists the activity in order while open. Each
  nested tool call is a card with its own open/closed state.
- An update that arrives before its delegation card moves into the card when
  the card arrives later in the turn. Anything still waiting when the turn or
  the session ends is shown in the conversation rather than dropped.
- The subagent's final report stays the delegation's own result, as before.
- This is the interim path until Knot takes subagents over ACP's dedicated
  subagent channel (#15).

### Non-goals

- Subagent sessions of their own (#15).
- Any change to how the main agent's own tool calls are shown.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `acp-panel-ui`: adds "A subagent's activity stays inside its delegation
  card".

## Impact

- `knot-acp`: `SessionUpdate::TextDelta` carries `_meta`.
- `knot` `panel_state`: a `subagent` module routes by
  `claudeCode.parentToolUseId`; `ToolCallCard` gains its nested items; card
  lookups by id reach nested cards.
- `knot` `panel_view`: the count line and nested activity in the tool-call card.

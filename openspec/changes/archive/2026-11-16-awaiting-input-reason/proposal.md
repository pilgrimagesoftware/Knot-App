# Proposal

## Why

An agent's cell showed "Awaiting input" while its last reply asked the user
nothing - replies that said outright that nothing needed the user's input were
flagged (#344). Two causes, and one gap that hid both:

- Autopilot's classifier counted status reports as questions, and its Mark
  and Ask effects put the agent in Awaiting input with no message, so the
  cell gave no hint that Autopilot had made the call, or why.
- A Panel agent's pending ACP permission request was taken off the panel only
  when the user answered it there or the turn ended. Answered any other way -
  `cancelled` by Stop, Deny as the session ended - it stayed pending, and the
  agent stayed in Awaiting input.
- Nothing on the cell said why any agent was waiting, so neither defect could
  be told from a real request.

## What Changes

- The classification prompt says what counts as asking the user: only a
  question or decision put to the reader. Status reports, "nothing needed from
  you", requests to other agents and closing courtesies are `completed`.
- An agent in Awaiting input carries the reason it is waiting while it waits:
  a permission request's title, a hook's message, or Autopilot's reading of
  the reply. The state dot's tooltip and the dashboard card show it, the
  notification carries it, and leaving Awaiting input clears it.
- The ACP client reports every permission request it answers, however it was
  answered, and the panel drops that request by id. An ended session has none
  pending.

## Impact

- `knot-autopilot`: classification prompt; `Effect::Mark` carries the
  classification.
- `knot-mcp-tools`: `land` records the reason; the hook tracker's sink records
  the hook message.
- `knot-agents`: `Agent::awaiting_reason`, `input_reason`,
  `AgentStore::set_awaiting_reason`; `set_state` clears it on leaving Input.
- `knot-acp`: `SessionEvent::PermissionResolved`.
- `knot`: the panel fold, the panel tracker's sink, the state dot's tooltip,
  the dashboard card.

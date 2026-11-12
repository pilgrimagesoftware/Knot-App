# Proposal

## Why

Knot threw away the ACP `stopReason`. The panel ended the turn the same way
whatever the reason, so a turn the agent stopped early (out of output tokens,
out of model requests, refused) looked exactly like the #22 symptom: an agent
that announced work and went idle. Nothing in the UI or the log could tell the
two apart.

## What Changes

- The client keeps the stop reason as a type (`StopReason`, the spec's values
  plus Knot's own `error` for a failed prompt and an `Other` for anything
  unnamed), and logs it beside the `session/prompt` response line.
- A turn that ends with `max_tokens`, `max_turn_requests`, `refusal` or an
  unnamed reason adds a notice line to the conversation saying so. `end_turn`
  adds nothing; neither does `cancelled`, which is the user's own Stop, nor
  `error`, which the conversation already shows as the error itself.

## Impact

- `knot-acp`: `StopReason`; `SessionUpdate::TurnEnd` carries it.
- `knot`: the panel's fold adds the notice; four catalogue keys.
- Specs: ADDED `acp-client` "A turn's stop reason is kept" and `acp-panel-ui`
  "A turn the agent stopped early says why". The existing "Streaming session
  updates" text, "turn-end (with a stop reason)", stays true and is not
  modified.
- Not changed: the spec-external `turn_end` session update is still parsed.
  The panel's replay tests drive a turn end through it, so removing it means
  reworking them, which is a follow-up rather than part of this change.
- Not changed: the agent's status line. A stopped turn still leaves the agent
  Idle, and the notice is in its conversation.

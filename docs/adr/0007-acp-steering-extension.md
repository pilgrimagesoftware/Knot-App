---
id: ADR-0007
title: Steer mid-turn prompts through the `_session/steering` ACP extension
status: proposed
date: 2026-11-13
tags: [acp, protocol, panel]
---

# ADR-0007: Steer mid-turn prompts through the `_session/steering` ACP extension

## Context

#26 asks for a prompt typed while an agent is working to reach that agent
mid-turn, as Claude Code's own queued input does. Knot's panel queues such a
prompt and sends it when the turn ends (`acp-panel-ui` "Prompt queue").

Stable ACP has no way to do it. A turn is one `session/prompt` that ends with
its response (<https://agentclientprotocol.com/protocol/prompt-turn>); the
only thing a client can send into a running turn is `session/cancel`.
`.claude/rules/protocol-conformance.md` exists because four bugs came from
modelling ACP on inference rather than on the spec, so depending on anything
outside the spec needs a recorded reason.

What the adapters actually expose, checked against their sources on
2026-11-13:

- `claude-agent-acp` 0.101.0 (`dist/acp-agent.js`): a `_session/steering`
  request (`STEER_METHOD`, `parseSteerRequest`, `steer()`), advertised as
  `InitializeResponse._meta.steering.supported: true`. A steer pushes the
  message onto the running turn's streaming input at priority `now`, or
  `later` while a permission is pending. With
  `_meta.steering.idleBehavior: "promptRequired"`, a steer that finds no
  running turn answers `{"outcome": "promptRequired"}` and starts nothing.
- `@agentclientprotocol/codex-acp` 2.5.0: the same request
  (`SESSION_STEERING_METHOD`, `executeOrQueueSteeringRequest`), backed by the
  Codex app-server's `turn/steer`, with the same advertisement.
- opencode's ACP agent
  (<https://github.com/anomalyco/opencode/blob/main/packages/cli/src/acp/agent.ts>)
  registers `_session/steering`.
- Gemini CLI (<https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/acp/acpSession.ts>,
  `prompt()`): a second `session/prompt` aborts the one in flight. Its "model
  steering" is an experimental TUI setting with no ACP surface.
- ACP v2's prompt RFD
  (<https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/rfds/v2/prompt.mdx>)
  lets a client send `session/prompt` while foreground work is active, and has
  the agent insert or hold it. That is the standard form of the same feature;
  it is a draft, not yet in the protocol or the adapters.

## Decision

Knot steers a queued prompt into the running turn with `_session/steering`,
and only under these conditions:

- The agent advertised `_meta.steering.supported: true` in its `initialize`
  response. Knot reads the capability; it never infers it from the agent
  type. An agent that did not advertise it is never sent the request.
- Every steer sets `_meta.steering.idleBehavior: "promptRequired"`, so a steer
  that finds no running turn hands the prompt back instead of starting a turn
  Knot cannot track. Knot then sends it through its own queue as the next
  turn.
- Steering is a per-prompt action ("Steer now"), not what Send does. Send
  still queues, and an agent without the capability keeps the queue
  unchanged.

The extension's wire shapes live in one module, `knot-acp`'s
`protocol/steering.rs`, citing the adapter source they were taken from.

## Options considered

- **`_session/steering`, capability-gated (chosen).** Three of the adapters
  Knot runs implement it with one wire contract. Gating on the advertised
  capability and falling back to the queue means an adapter that drops or
  changes the extension degrades to today's behaviour rather than breaking.
- **Cancel and re-prompt.** Send `session/cancel`, wait for the turn to end
  `cancelled`, then send the new prompt, possibly with a summary of what was
  under way. Works with any adapter, but throws away the turn's in-progress
  work and context. It is the opposite of what #26 asks for: Claude Code's
  queued input joins the turn, it doesn't end it. Rejected.
- **Knot-side emulation.** Hold the prompt and have Knot inject it, for
  example as an MCP tool result or a nudge the agent is told to poll for.
  It would need the agent's cooperation on every turn, change what the model
  sees in ways the user didn't write, and still couldn't interrupt a
  generation. Rejected.
- **Wait for ACP v2.** No dependence on an extension, but the prompt RFD is a
  draft with no adapter shipping it, and the feature is available today.
  Rejected for now; it is this decision's exit condition (below).

## Consequences

- Claude, Codex and opencode agents get Steer now. Gemini does not, nor does
  any agent that doesn't advertise the extension; their queue is unchanged.
  Copilot's adapter was not available to check, and the capability check
  covers it whichever way it goes.
- Knot depends on a non-standard request. If an adapter changes its shape,
  the steer fails: the prompt is marked failed in the queue, as a failed
  delivery is, and the user can retry, edit or delete it. If an adapter stops
  advertising it, Steer now disappears. Neither loses the prompt. The shapes
  are pinned in one module with tests built from `claude-agent-acp`'s own
  replies, so a change shows up in one place.
- An adapter that ignores `promptRequired` and answers `startedNewTurn` starts
  a turn Knot did not send and won't see end. Knot counts the prompt as
  delivered rather than send it twice. It is a known, documented gap.
- **Exit condition.** When ACP v2's prompt RFD lands in the protocol and the
  adapters Knot runs implement it, Knot moves to the standard mid-turn
  `session/prompt` and drops `_session/steering`. That is a new ADR
  superseding this one.
- Pushing running work into the background (the rest of #26) has no request
  in the spec, the extension or the v2 background-work RFD
  (<https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/rfds/v2/background-work.mdx>),
  so this decision does not cover it.

# Design

## Sources

- `claude-agent-acp` 0.101.0, `dist/acp-agent.js`:
  - `STEER_METHOD = "_session/steering"` and `steer()`: a running turn means an
    unsettled entry in `turnQueue`. The message is pushed onto the SDK's
    streaming input at priority `now`, or `later` while a permission or
    elicitation is open.
  - `parseSteerRequest`: `idleBehavior`, if present, must be
    `"promptRequired"`.
  - `initialize()`: `_meta.steering.supported: true`.
- `@agentclientprotocol/codex-acp` 2.5.0: `SESSION_STEERING_METHOD`,
  `executeOrQueueSteeringRequest`, which uses the app-server's `turn/steer`.
- opencode: `packages/cli/src/acp/agent.ts` registers `_session/steering`.
- Gemini CLI: `packages/cli/src/acp/acpSession.ts` `prompt()` calls
  `this.pendingPrompt?.abort()`, so a second prompt replaces the first.
- ACP RFDs: `docs/rfds/v2/prompt.mdx` (prompts during foreground work) and
  `docs/rfds/v2/background-work.mdx` (background work is agent-detached; the
  client can cancel an item but has no request to detach one).

## Wire shape

Request, sent only when `initialize` advertised `_meta.steering.supported`:

```json
{"jsonrpc":"2.0","id":7,"method":"_session/steering",
 "params":{"sessionId":"s1","prompt":[{"type":"text","text":"also check the tests"}],
           "_meta":{"steering":{"idleBehavior":"promptRequired"}}}}
```

Result: `{"outcome":"injected"}` while a turn runs, or
`{"outcome":"promptRequired","reason":"noRunningTurn"}` when none does.
`startedNewTurn` is what an adapter sends a client that does not opt in; Knot
always opts in, so it treats `startedNewTurn` as delivered too. Knot cannot
stop the adapter from starting that turn, and sending the prompt again would
duplicate it. The steered message's output streams as ordinary
`session/update`s inside the running turn. The `session/prompt` that started
the turn resolves only once the steer is answered, so `turn_active` stays
true throughout and needs no change.

## Why `promptRequired`

Without the opt-in, a steer that arrives just after the turn settles starts a
detached turn the client didn't ask for, outside its prompt lifecycle. That
turn has no `session/prompt` response, so Knot would never see it end.
`promptRequired` hands the prompt back, and Knot sends it through its normal
path (`drain_panel_prompt`), so every turn Knot shows is one it started.

## Per prompt, not by default

A steer at priority `now` aborts the current generation, and the cycle in
progress emits a result of its own. That is right when the user is
correcting course, and wrong when the user is just lining up the next task,
which the queue already handles. So Send keeps queueing, and steering is an
explicit action on a queued entry: Steer now, beside Send now
(`prompt_queue::send_now`), which is a different thing - it moves an entry to
the head past a usage-limit hold.

## States

A queued entry being steered is `steering`. It isn't sent twice, and it isn't
taken by `drain_panel_prompt` at turn end. The steer's answer decides what
happens next:
- `injected` / `startedNewTurn`: the entry leaves the queue and is recorded in
  the conversation as a user prompt inside the running turn.
- `promptRequired`: the entry goes back to the head of the queue, unmarked,
  and the turn end delivers it as today.
- An error: the entry stays, marked failed with the error, like a failed
  delivery.

## Not done: (b)

There's nothing to call. Steering into a running tool call backgrounds that
tool in `claude-agent-acp` (CLI 2.1.286), so for Claude, Steer now during a
long command is the nearest thing to Ctrl+B, and this change provides it.
A real "move this to the background" control waits for a request in ACP;
the v2 background-work RFD lists background work and cancels it, but has no
such request. Showing background work comes with v2 `state_update.background`.

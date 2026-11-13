# Proposal

## Why

#26 asks for two things Claude Code does in its own terminal: (a) picking up
a prompt the user types while the agent is working, mid-turn, rather than
after the turn ends; and (b) pushing the running task into the background so
the agent can carry on with something else.

Knot already queues a prompt sent during a response and delivers it when the
turn ends (`acp-panel-ui` "Prompt queue"), and Stop cancels the turn. What it
can't do is hand the agent a prompt while the turn is still running.

## What the adapters expose (checked against their sources)

- **Steering, for (a).** `claude-agent-acp`, `codex-acp` and opencode's ACP
  agent implement the `_session/steering` request, advertised as
  `InitializeResponse._meta.steering.supported: true`. A steer pushes the
  message into the running turn. Claude pre-empts the current generation, or
  slots the message in between tool calls; Codex uses its app-server's
  `turn/steer`. With `_meta.steering.idleBehavior: "promptRequired"`, a steer
  that finds no running turn answers `promptRequired` and leaves the prompt to
  the client.
- ACP v2's prompt RFD (`docs/rfds/v2/prompt.mdx`) moves the same way: a client
  may send `session/prompt` while foreground work is active, and the agent
  inserts or holds it. Steering is the v1 extension of that.
- **Gemini CLI** has no steering over ACP: a new `session/prompt` aborts the
  one in flight (`packages/cli/src/acp/acpSession.ts`, `prompt()`). Its model
  steering is an experimental TUI setting. Copilot's adapter could not be
  checked; detecting the capability covers it either way.
- **Backgrounding, for (b).** No adapter or spec has a client request that
  moves the running work to the background. The v2 background-work RFD
  (`docs/rfds/v2/background-work.mdx`) lists background terminals, tool calls
  and subagents, and adds per-item `tool_call/cancel` and `terminal/cancel`,
  but work becomes background only when the agent detaches it (Claude's
  `run_in_background`, its Ctrl+B in the TUI, a timeout). In
  `claude-agent-acp`, a steer that arrives during a foreground tool call
  moves that tool to the background, as a side effect. Its async-task
  reporting is gated on JetBrains' AIR capability.

## What Changes

- The client records whether the agent supports steering, from the
  `initialize` response's top-level `_meta`, and can send `_session/steering`
  with `idleBehavior: "promptRequired"`.
- A queued prompt for an agent that supports steering gains **Steer now**.
  Steer now hands the prompt to the running turn, and it leaves the queue. If
  the turn has already ended (`promptRequired`), Knot sends the prompt
  normally. If the steer fails, the prompt stays queued with the failure
  shown.
- The queue stays the default. Steering pre-empts the agent's current
  generation, so it is the user's choice per prompt, not what Send does.
- For an agent that doesn't advertise steering, nothing changes.
- (b) is not proposed: there is no protocol request to build it on. Its
  closest form today is steering into a running tool call, which this change
  delivers for Claude without needing anything further.

## Impact

- `knot-acp`: keep `InitializeResult`'s top-level `_meta`;
  `AgentCapabilities::supports_steering`; `AcpClient::session_steer` returning
  the outcome (`injected`, `startedNewTurn`, `promptRequired`).
- `knot-terminal` / `knot`: an `AcpSession` steer pass-through; a queue
  entry's Steer now action, beside Send now and the others
  (`workspace_window/prompt_queue.rs`, `workspace_window/panel/prompt.rs`
  `deliver_panel_prompt` / `drain_panel_prompt`).
- Specs: ADDED `acp-client` "Steering a running turn" and `acp-panel-ui`
  "Steer a queued prompt into the running turn". "Prompt queue" is not
  modified: a steered prompt leaves the queue, and the queue keeps its
  behaviour for everything still in it.
- Size: medium. The protocol part is small; most of the work is the queue
  action, its states, and tests against a fake adapter.

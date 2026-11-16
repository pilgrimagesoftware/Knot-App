# Proposal

## Why

Users learn an agent has run out of its usage allowance only when a turn is
refused (#291). Claude Code's own app warns beforehand - "Approaching session
usage limit · Resets at 10:10 PM · Get more usage" - and Knot should where it
can.

## What Changes

- For Codex agents, Knot reads the account's rate-limit snapshot - the 5-hour
  and weekly windows' used percentage and reset time - from the latest
  `token_count` event in Codex's session files (`$CODEX_HOME`, else
  `~/.codex/sessions`), off the main thread, once a minute while a Codex agent
  is in the window.
- At 75% or more of a window that has not reset since, a banner over the
  selected Codex agent reads "Approaching usage limit", says when the window
  resets, and offers "Get more usage" (Codex's usage settings). Dismissing it
  holds until that reset.
- Missing files or a changed format show nothing, never an error.

## Not added

- **Claude.** Claude Code reports these figures only to a status-line command
  (Knot does not write to the user's Claude settings) and its ACP adapter does
  not forward them: `claude-agent-acp` #625 is open, and the PRs that would have
  forwarded `rate_limit_event` (#508, #568) were closed unmerged.
- **Codex over ACP.** `codex-acp` #227, to forward the snapshot over ACP, is
  open. When either adapter ships it, this reads from there instead.

## Impact

- `knot`: `codex_usage` (reader), the window's `codex_usage` cache, its source
  seam and the repaint-poll term, `render/usage_banner`; constants in `consts`.
- `knot-core`: `agent_type::CODEX`.

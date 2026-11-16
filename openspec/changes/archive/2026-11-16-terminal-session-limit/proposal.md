# Proposal

## Why

Claude Code may grant a session that reaches its usage limit a wrap-up
allowance: it prints "Usage limit reached · wrapping up", finishes what it was
doing, and stops. A Terminal-mode agent showed none of that - its output kept
it Working, the Stop hook turned it Idle, and the next prompt was refused
(#36). Panel-mode agents already show Session limit from the turn's error
(#318, #368); the ACP adapter forwards no wrap-up signal, so nothing more is
needed there.

## What Changes

- A Claude agent's terminal output is scanned for the usage-limit notices
  ("Usage limit reached", "You've hit your session limit", "You've hit your
  usage limit") at the start of a line. Seeing one puts the agent in the same
  red Session limit status a Panel agent gets, with its reset time when the
  notice gives one.
- The status holds through the wrap-up: further output, tool-use hooks and
  the closing Stop do not move it, and Autopilot does not answer that Stop.
  A permission prompt during the wrap-up shows as Awaiting input over it.
  The next prompt the user submits lifts it, and the same notice after that
  is reported again.

## Impact

- `knot-terminal`: `SessionLimitTap`, fed beside the pull request scan;
  `TerminalSession::take_session_limit`.
- `knot-agents`: `AgentStore::set_tracked_state`, the write terminal trackers
  and hooks use, which keeps a shown limit.
- `knot-mcp-tools`: the `UserPromptSubmit` hook clears the limit; hook states
  go through `set_tracked_state`; Autopilot skips a limited agent.
- `knot`: the poll drains each terminal session's limit into the store.
- `knot-core`: `agent_type::CLAUDE`.
- Other terminal agent types are not scanned: nothing in Knot sees them
  submit a prompt, so a limit would hold until the shell exited.

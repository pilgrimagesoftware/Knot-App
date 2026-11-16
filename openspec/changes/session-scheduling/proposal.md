# Proposal

## Why

Some agents run scripted workflows: a startup prompt that should run at a
set time of day or on a recurrence. Knot can only start them by hand. And an
agent left idle keeps its session, its adapter process and its context until
someone notices (#404).

## What Changes

**Scheduled start**

- An agent with a startup prompt can carry a schedule. When it comes due,
  Knot opens the agent's workspace window if needed and starts the agent in a
  new conversation, so the startup prompt runs from a clean context.
- A schedule is either a **preset** (every day, weekdays, chosen days, at a
  time; or every N hours) or a five-field **cron expression**. Both evaluate
  in the Mac's local time zone.
- An agent that is mid-turn when its run comes due is not interrupted; that
  run is skipped and recorded as skipped.
- Knot runs schedules only while it is running. A run missed by up to
  10 minutes (the Mac was asleep, or Knot was launching) still runs once on
  wake or launch; an older miss is skipped and shown as missed. Missed runs
  are never run in a batch.

**Idle shutdown**

- An agent can be **deactivated** or **reset** (restarted in a new
  conversation) after it has been idle for a set time. Off by default. A
  settings default applies to every agent; each agent can inherit it, turn
  it off, or set its own time and action.
- Idle means all of: no turn in progress, no running subagents, no queued
  prompt, no pending permission request, no running background process under
  the session beyond its own adapter or shell, and no user activity in the
  agent's conversation or terminal (typing, sending, scrolling) for the whole
  period. "Awaiting input" counts as idle: the user has not answered.
- **A reset after idling does not run the startup prompt.** Otherwise a
  scripted agent would run its script, go idle, be reset, run it again, and
  never stop. Only a scheduled start, or a start the user makes, runs it.

**Indicators**

- Sidebar row: a clock detail line naming the next run ("Next run 09:00"),
  and, in the last part of an idle countdown, a timer detail line
  ("Idle 25 min · deactivating in 5 min"). The compact sidebar shows the same
  in its tooltip. The workspace header shows a small timer icon for the
  selected agent while its countdown is showing, with the same text as its
  tooltip.
- Nothing is shown for an agent with neither feature, and the timer is not
  shown for most of an idle period, so the sidebar does not fill with
  countdowns.

**Not included**

- Free-text schedules ("every weekday at 9am"). Presets cover what that
  would, without a parser for a language; cron covers the rest.
- Running schedules while Knot is not running (a launchd agent). Knot is a
  windowed app; a background launcher is a separate decision.
- Shell agents: neither feature applies. A shell has no startup prompt, and
  "idle" for a shell someone left open is not something Knot can judge.

## Impact

- New capability `session-scheduling`.
- `agent-list-ui`: indicators. `agent-editor-ui`: schedule and idle fields.
  `settings-ui`: the idle default. `settings-persistence`: the idle default
  is a preference.
- `knot-core`: `SavedAgent` gains `schedule` and `idle_policy` (both
  `#[serde(default)]`); `Settings` gains the idle default.
- `knot`: an app-level schedule ticker; idle evaluation in the workspace
  window's poll; a per-agent last-user-activity stamp; low-rate process
  sampling for agents with an idle policy.
- Dependency decision (ADR candidate): cron evaluation. The workspace uses
  `jiff` and has no `chrono`; the common cron crates depend on `chrono`. See
  `design.md`.
- `settings-persistence`'s "Preferences and durable data are stored
  separately" is also MODIFIED by the unarchived `dismiss-unused-mcp-servers`
  change. Whichever is archived second must merge the preferences list.

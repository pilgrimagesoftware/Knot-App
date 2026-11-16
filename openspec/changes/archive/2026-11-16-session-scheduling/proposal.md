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
- A stopped agent (stopped by the user, or deactivated for idling) is
  activated, then started. A busy agent (mid-turn, a pending permission
  request, a queued prompt, or an unanswered message it holds) is not
  interrupted: that run is skipped and recorded with the reason.
- The workspace window opens without taking focus from another app.
- Knot runs schedules only while it is running. A run missed by up to
  10 minutes (the Mac was asleep, or Knot was launching) still runs once on
  wake or launch; an older miss is skipped and shown as missed. Missed runs
  are never run in a batch. The last run started is persisted per agent, so a
  relaunch never runs the same one twice.

**Idle shutdown**

- An agent can be **deactivated** or **reset** (restarted in a new
  conversation) after it has been idle for a set time. Off by default. A
  settings default applies to every agent; each agent can inherit it, turn
  it off, or set its own time and action.
- Idle means all of: no turn in progress, no running subagents, no queued
  prompt, no pending permission request, no running background process under
  the session beyond its own adapter or shell, and no user activity in the
  agent's conversation or terminal (typing, sending, scrolling) for the whole
  period. An agent holding a message it hasn't answered is not idle. A turn
  that ended asking the user a question counts as idle, since nobody
  answered. A pending permission request does not.
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
- New dependency: `croner` 4 with only its `jiff` feature, so no `chrono`
  (ADR-0008). Cron evaluation is not hand-rolled.
- `settings-persistence`: "Preferences and durable data are stored
  separately" is also MODIFIED by the unarchived `dismiss-unused-mcp-servers`
  change, so whichever is archived second must merge the preferences list.
  "Per-workspace UI state is its own document" is MODIFIED here to hold the
  last run started.
- **Depends on** the unarchived `keep-briefs-across-reset` change
  (`agent-lifecycle`: "Knot's own reset waits for open threads"). Both
  automatic paths follow it by treating an unanswered message as busy, not
  idle, rather than exempting themselves. If that change were dropped, this
  one's "unanswered message" conditions would need their own definition.

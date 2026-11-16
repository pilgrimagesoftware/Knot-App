# Design

## Data

`SavedAgent` (`knot-core/src/settings/records.rs`) gains two fields, both
`#[serde(default)]` so older records load unchanged:

```rust
pub schedule: Option<Schedule>,
pub idle_policy: IdlePolicy,          // default: Inherit

pub enum Schedule {
    Preset(Preset),
    Cron(CronExpr),                   // parsed at load; invalid -> kept as text, never fires
}
pub enum Preset {
    Daily { at: LocalTime },
    Weekdays { at: LocalTime },
    Days { days: WeekdaySet, at: LocalTime },
    EveryHours { hours: NonZeroU8 },  // on the hour, from midnight
}
pub enum IdlePolicy { Inherit, Off, Custom { after: Duration, action: IdleAction } }
pub enum IdleAction { Deactivate, Reset }
```

`Settings` gains `idle_default: Option<IdleDefault { after, action }>`,
`None` (off) by default. Each is a closed vocabulary with `Display` /
`FromStr` and is stored as its own string form, per the vocabulary rule.

A schedule without a startup prompt is rejected by the editor: there is
nothing to run.

## Next occurrence

One pure function, the only place a schedule is evaluated:

```rust
fn next_run(schedule: &Schedule, after: jiff::Zoned) -> Option<jiff::Zoned>
```

Presets and cron both go through it, so both get the same time zone and DST
rules: a local time skipped by a spring-forward runs at the first valid time
after it; a time repeated by a fall-back runs once.

### Cron evaluation (dependency decision, ADR candidate)

The common crates (`cron`, `croner`) depend on `chrono`, which the workspace
does not use; it already has `jiff`. Recommendation: a small evaluator for
standard five-field cron (`*`, lists, ranges, steps, names for months and
days, the Vixie day-of-month / day-of-week OR rule) over `jiff`, with
property tests against a brute-force minute-by-minute search. About 200
lines, no new dependency. The alternative, `croner` plus `chrono`, is
recorded in the ADR if the reviewer prefers it.

## Scheduled start

Sessions belong to workspace windows, and the only per-agent tick is the
window's `repaint_poll_tick`. A schedule must fire with the window closed,
so it needs an app-level ticker:

- One task at app level (like `adapter_updates`), waking every 30 seconds.
  It reads saved agents' schedules from the shared `AgentStore`, compares
  each `next_run` with now, and on a due run defers onto the main thread.
- On the main thread: open the agent's workspace window through the path a
  Command Center card takes (`open_agent`), but **without taking focus**:
  the window is shown and ordered in front of Knot's other windows only,
  and Knot is not activated over another app. If Knot is already the
  frontmost app, the window comes forward as a card click would.
- `restart_agents` only starts agents that are activated (its doc:
  "an agent that is not activated is only reset"). A scheduled run is the
  case where the agent is usually stopped, so the run first marks it
  activated with `store.set_activated(id, true)` alone, then calls
  `restart_agents(&[id], keep_conversation = false)`. That starts exactly one
  fresh session and queues the startup prompt (`PromptOrigin::Startup`) once,
  as a manual start does. Not `activate_agent`: that also calls
  `start_sessions`, so following it with a restart would spawn an adapter,
  tear it down at once and spawn another, and could register the agent and
  queue its startup prompt twice.
- **Busy** at fire time means any of: a turn in progress, a pending
  permission request, a non-empty prompt queue, or an unanswered message the
  agent holds. A new conversation would discard each of those. The run is
  skipped and recorded with that reason.
- Runs are keyed by their scheduled instant. The last fired instant per
  agent is **persisted** in the workspace's UI-state document (Knot recorded
  it; the user did not configure it). So a relaunch inside the grace window
  never fires the same run again, and a slow tick never fires two.
- **Grace**: a due run up to 10 minutes late still fires (sleep, launch).
  Older: marked missed, the next occurrence takes over. At most one run
  fires per agent per tick.

## Idle shutdown

Evaluated in the workspace window's poll, which owns the sessions, but at
most once every 15 seconds per window rather than every 33 ms frame.

An agent's idle start is the latest of:

| Signal | Source | New? |
| --- | --- | --- |
| turn ended / went idle | `Agent.idle_since` | exists |
| subagents running | `knot-subagents` registry `running_count` | exists; stamp when it drops to 0 |
| queued prompt / pending permission | panel queue, session state | exists; idle only while empty |
| user activity | composer keys, sends, terminal keys, scroll in conversation or terminal | **new** `last_user_activity` per agent, set on the main thread by the existing handlers |
| background processes | `knot-processes` sample of the session's tree | **new** sampling for agents with an active idle policy |

Process sampling today runs only while a processes pane is open
(`claim_process_sample`). Agents with an idle policy get their own sample,
once a minute, through the same `spawn_blocking` path, and its result joins
`repaint_poll_tick`'s chain like the existing sample. A process other than
the session's own adapter or shell counts as activity.

An agent is never idle while it has a pending permission request, or holds a
message it has not answered (the open-threads rule below). A turn that ended
asking the user a question (`AgentState::Input` with no permission pending)
does count as idle.

When `now - idle_start >= after`:

- `Deactivate`: `deactivate_agent`, the context menu's action.
- `Reset`: `restart_agents(.., keep_conversation = false)` with the startup
  prompt suppressed for that restart, as a field on the restart request, so
  the suppression cannot leak to a later manual or scheduled start.

## Interaction with `keep-briefs-across-reset` (#405)

That unarchived change's `agent-lifecycle` requirement "Knot's own reset
waits for open threads" says a fresh-conversation restart Knot starts by
itself waits while the agent holds an unanswered message. Both automatic
paths here are that kind of restart. Both treat an unanswered message as
"not done", in line with that rule rather than as an exemption from it:

- Idle reset and idle deactivate: an agent holding an unanswered message is
  not idle, so neither fires until it answers.
- Scheduled start: such an agent is busy, and the run is skipped with that
  reason.

When either path does restart an agent, #405's handoff runs as for any
fresh-conversation restart, with the note in the agent's first turn. Since
both paths require the agent to hold nothing unanswered, the handoff has no
briefs to reopen. It also triggers none of the sender notices (#433, "your
message was reopened"), because those go only to senders of messages the
agent held unanswered. An idle deactivate keeps the conversation, so it
starts no handoff at all.

The new per-agent runtime fields (`last_user_activity`, last fired run,
idle sample) go in one struct per agent, pruned in `teardown_session`, not
three more parallel maps.

## Indicators

Built from the `AgentRow` snapshot, so drawing reads no clock and no I/O
beyond `now`, which rows already use for durations.

- Clock line: `detail_line(IconName::Clock, "Next run …")` whenever a
  schedule exists. "Missed 09:00" for an hour after a missed run.
- Timer line: shown once the remaining time is the smaller of 10 minutes or
  half the period. Text: idle so far, action, time left.
- Compact sidebar: the same lines in `compact_tooltip`.
- Title bar: a timer icon beside the state dot for the selected agent while
  its timer line shows, tooltip with the same text.

## Testing

- `next_run`: presets, cron fields, DST gaps and repeats; cron property test
  against brute force.
- The ticker's decision as a pure function of (schedules, last fired, now,
  busy): fires once, skips busy, grace window, missed, never two.
- Idle start as a pure function of the signals; each signal resets it.
- The reset path does not queue the startup prompt; a scheduled start does.
- Persistence: older records load as no schedule, `Inherit`.
- Catalog keys resolve; times and counts survive substitution.

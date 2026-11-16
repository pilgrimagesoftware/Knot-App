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
- On the main thread: `WorkspaceWindow::open_agent` (the path a Command
  Center card takes) to open or raise the window, then
  `restart_agents(&[id], keep_conversation = false)` on it, which starts a
  fresh session and queues the startup prompt (`PromptOrigin::Startup`) as a
  manual start does.
- Mid-turn at fire time: skipped, recorded as skipped.
- Runs are keyed by their scheduled instant. The last fired instant is kept
  per agent (runtime state, not persisted), so a tick never fires the same
  run twice and a slow tick never fires two.
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

When `now - idle_start >= after`:

- `Deactivate`: `deactivate_agent`, the context menu's action.
- `Reset`: `restart_agents(.., keep_conversation = false)` with the startup
  prompt suppressed for that restart, as a field on the restart request, so
  the suppression cannot leak to a later manual or scheduled start.

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

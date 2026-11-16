# Design

## Where the guard lives

The issue suggested a per-agent "spawn in flight / running" registry checked
on every spawn path. The check that exists already does that job: all four
paths (message wake-up, resume/reconnect, restart, plan dispatch) end in
`ensure_panel_session`, which returns while `panel_sessions` holds a slot for
the agent. An agent belongs to exactly one workspace (`move_to_workspace`
removes it from its source), and a workspace has at most one window, so a
second window can't start the same agent either.

What failed was letting go of a slot. Removing a slot from `panel_sessions`
reopened the gate for the next start, but the task still filling that slot
was never told. A second registry would have the same gap unless its
release also cancelled the in-flight connect. So the fix is in the slot's
lifecycle, not in a new map.

## The retirement signal lives in the slot

`Connecting` holds both ends of a `oneshot` channel. `connect_into` takes the
receiver when it starts, and the sender stays in the variant. Replacing the
variant drops the sender, which fires the receiver. That covers `retire()`
and the attempt's own outcome alike. So retiring a slot cancels its connect
by construction, and no teardown path can forget a "cancelled" flag.

Alternatives considered:

- **Abort the connect task's `JoinHandle`.** This needs the handle stored next
  to the slot: a twelfth parallel per-agent map, which `rust-structure.md`
  warns against. It also leaves the publish race below unsolved.
- **Poll a flag during the connect.** Adds latency and a timer for something a
  channel already does.

## Cancellation kills the adapter

The connect phase is one future, `prepare`: spawn, initialize, session load
or new, config replay and startup-prompt expansion. `connect_into` races it
against the signal with `select!`. Dropping `prepare` drops the only strong
`Arc<Transport>`. Every background pump holds the transport through a `Weak`
(the cycle fixed earlier), and the transport's `Child` has `kill_on_drop`, so
the adapter dies with the future.

## The publish is a critical section

When the connection finishes there is no signal left to fire. A retire can
still land between the end of `select!` and the publish. So publishing checks
for `Connecting` and writes the outcome under one slot lock
(`PanelSessionSlot::publish`). If the retire lands first, the outcome comes
back and its handle is stopped. If it lands second, `retire()` takes the
`Ready` handle and the window stops it. No order of the two leaves a live
adapter in a slot that nobody holds.

`on_session_id` runs after a check of the same kind, so a retired attempt
doesn't write its session id over the one that replaced it. A retire that
lands between that check and the publish can still let the id through. That
is harmless: the id is only a resume hint, the attempt is stopped at the
publish, and the replacement writes its own id after it connects. Running the
callback under the slot lock would close this gap, but the callback writes
the roster file, and the render path reads the slot every frame.

## Bounded overlap

`kill_on_drop` sends SIGKILL when the future is dropped, and the runtime drops
an aborted future on its next poll. A replacement started in the same tick
can spawn while the retired adapter is being killed. The overlap is one
process exit, not a second agent: the retired adapter can't publish and can't
send a turn.

# Design

## Which agents are running: `has_session`

Today a card's "running" is `DashboardAgent.is_running`, filled from
`agent.activated` (`command_center.rs:263`, `render/overview.rs:80`). That
is the wrong signal for both pieces here. Closing a workspace window drops its
sessions (`impl Drop for WorkspaceWindow`) without clearing `activated`, so in
the Command Center such an agent reads as running in its last stored state,
while there is no session to send to.

The snapshot field is replaced by `has_session`: the agent is activated, its
workspace window is open, and that window holds a session slot for it (a
panel session connecting or ready, or a terminal session). A connecting slot
counts because `deliver_panel_prompt` queues into it and the queue drains
when the session is ready. A failed slot does not: `deliver_panel_prompt`
would queue into it too, but nothing drains that queue, so the card shows
the agent as not running and `quick_prompt` refuses rather than accept a
prompt that would never go.

- **Workspace dashboard**: read from its own `panel_sessions` / `sessions`.
- **Command Center**: `WindowRegistry::workspace_view` for the agent's
  workspace; `None` (window closed) means no session. Otherwise it reads that
  view's maps through a `pub(crate) fn has_session(&self, id) -> bool` on
  `WorkspaceWindow`. That is an in-memory read of another entity, not I/O.

The card's working indicator takes `has_session` too, so an agent in a closed
window now shows "not running" on its card as well as in the summary. That
is a behavior change to the existing card, and the reason the summary and the
cards stay in agreement. The snapshot is rebuilt when the store revision
changes. A window opening or closing must also trigger a rebuild, and that
trigger is added to the Command Center's existing store poll.

## Status summary

A pure function over the same `DashboardAgent` snapshots the cards render
from, so it reads nothing the cards do not and costs a pass over a roster:

```rust
fn status_counts(agents: &[DashboardAgent]) -> Vec<(WorkingIndicatorState, usize)>
```

It maps each non-shell agent through `working_indicator::state(state,
has_session)` — the function that picks the card's indicator — so the summary
and the cards cannot disagree. Order: awaiting input, working, idle, error,
not running; zero counts are dropped. Each entry is a colored dot from the
card's `StatusPalette` and a count with its label through `l10n::pluralize`
(one key pair per state), since "1 Error" / "2 Errors" is the translator's
to word.

Where it is drawn depends on how many sections a dashboard has. The Command
Center shows several workspaces: one summary in its header over every agent,
and one beside each section's name. A workspace window's dashboard shows a
single section, so it draws only that section's summary. A header summary
there would repeat it word for word. Nothing new is polled: the snapshots are already rebuilt on
the store revision and spinner ticks that repaint the grid.

## Quick prompt

### Where the text lives

One `InputState` per panel agent card, owned by the view that draws the grid
(the workspace window's dashboard, or `CommandCenterWindow`), in a single
`BTreeMap<Uuid, Entity<InputState>>`. It is created when a card first draws
and pruned in the same place the view drops agents that left its roster
(the "parallel per-key maps" rule: one map, pruned in one function).

### Delivery

Both surfaces end in `WorkspaceWindow::deliver_panel_prompt(id, text,
PromptOrigin::User)`, which records the prompt in the conversation, sends it
when the agent is free, and queues it when it is not — the composer's path,
not a second one.

`deliver_panel_prompt` stays `pub(in crate::workspace_window)`. A narrow
`pub(crate) fn quick_prompt(&mut self, id, text) -> bool` on
`WorkspaceWindow` calls it with `PromptOrigin::User`, so the Command Center
gets one entry point and not the composer's internals.

- **Workspace dashboard**: the card's own window owns the session, so it
  calls `quick_prompt` on itself.
- **Command Center**: it holds no sessions. It resolves the owning
  workspace (`AgentStore::workspaces`) and that window's view through
  `WindowRegistry::workspace_view`, then calls `quick_prompt` on it. With
  the window closed, `has_session` is false, the field is disabled, and
  there is nothing to route. A send that races the window closing finds no
  view, which is handled like no session: the text stays.

`deliver_panel_prompt` returns `false` for an agent with no panel session.
The field keeps its text in that case rather than clearing it, so a send that
raced the session ending loses nothing.

Cmd+Enter delivers, then calls `WorkspaceWindow::open_agent`, the path a card
click already takes.

### State shown

| Agent | Field |
| --- | --- |
| shell agent | not drawn |
| not running | disabled, "not running" placeholder |
| idle or error, no session limit | enabled, "Send a prompt…" |
| working, awaiting input, or held by a session limit | enabled, "Will queue until it can take it" |

One rule: the field says it will queue whenever `deliver_panel_prompt` would
queue. That covers a turn in progress, a pending permission request (shown as
awaiting input, and queued behind "Permission gate is independent of the
queue"), and a session-limit hold (#368). The placeholder comes from the
snapshot the card already has (`state`, `session_limit`, `has_session`), so
choosing it is not I/O.

### Card height

`CARD_HEIGHT` is a fixed 124px used for grid layout. It grows by the field's
height for every card, shell cards included, so rows stay level; a shell
card leaves the space empty rather than changing height.

### Keys and focus

The field stops click propagation, so clicking into it does not trigger the
card's "open workspace" tap. Enter and Cmd+Enter are handled on the input's
own events; nothing global is bound.

Taller cards mean fewer fit in the Command Center's default size. That is
what its existing "cards are never shrunk or dropped" rule already handles:
the content scrolls.

## Alternatives considered

- **Disable the field while the agent works (Swift).** Rejected: Knot queues
  prompts, and a field that refuses while the composer for the same agent
  accepts would make two rules for one action.
- **Command Center opens the workspace window to send.** Rejected: it would
  turn every quick send into a window switch, which is what Cmd+Enter is for.
- **Allow shell agents.** Deferred, not specified: writing to a shell needs
  its own rules (what Enter means, whether a command line is in progress)
  and is not what this issue asks for.

## Testing

- `status_counts`: order, dropped zeros, shell agents excluded, agents
  without a session counted as not running whatever their stored state.
- `has_session` in the Command Center: window open with a slot, window open
  without one, window closed with `activated` still set.
- The field's state table, as a pure function of the snapshot.
- Delivery: a window test that a quick prompt to a busy agent lands in its
  queue with origin `User`, and to an idle agent is recorded and sent; a
  Command Center test that the prompt reaches the owning window.
- Catalog keys resolve; the count survives substitution.

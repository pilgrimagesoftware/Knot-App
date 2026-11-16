# Design

## Status summary

A pure function over the same `DashboardAgent` snapshots the cards render
from, so it reads nothing the cards do not and costs a pass over a roster:

```rust
fn status_counts(agents: &[DashboardAgent]) -> Vec<(WorkingIndicatorState, usize)>
```

It maps each non-shell agent through `working_indicator::state(state,
is_running)` — the function that picks the card's indicator — so the summary
and the cards cannot disagree. Order: awaiting input, working, idle, error,
not running; zero counts are dropped. Each entry is a colored dot from the
card's `StatusPalette` and a count with its label through `l10n::pluralize`
(one key pair per state), since "1 Error" / "2 Errors" is the translator's
to word.

The dashboard header passes every agent it shows; each workspace section
passes its own. Nothing new is polled: the snapshots are already rebuilt on
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

- **Workspace dashboard**: the card's own window owns the session, so it
  calls `deliver_panel_prompt` directly.
- **Command Center**: it holds no sessions. It resolves the owning
  workspace (`AgentStore::workspaces`) and that window's view through
  `WindowRegistry::workspace_view`, then calls `deliver_panel_prompt` on it.
  Panel sessions live in an open workspace window, so if that window is not
  open the agent has no session; its card reports not running and the field
  is disabled, so there is nothing to route.

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
| idle / awaiting input / error | enabled, "Send a prompt…" |
| working | enabled, "Queues until the turn ends" |

The placeholder comes from the snapshot the card already has, so choosing it
is not I/O.

### Card height

`CARD_HEIGHT` is a fixed 124px used for grid layout. It grows by the field's
height for every card, shell cards included, so rows stay level; a shell
card leaves the space empty rather than changing height.

### Keys and focus

The field stops click propagation, so clicking into it does not trigger the
card's "open workspace" tap. Enter and Cmd+Enter are handled on the input's
own events; nothing global is bound.

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

- `status_counts`: order, dropped zeros, shell agents excluded, not-running
  agents counted as not running whatever their stored state.
- The field's state table, as a pure function of the snapshot.
- Delivery: a window test that a quick prompt to a busy agent lands in its
  queue with origin `User`, and to an idle agent is recorded and sent; a
  Command Center test that the prompt reaches the owning window.
- Catalog keys resolve; the count survives substitution.

# Proposal

## Why

The Swift dashboard had two pieces the port never specified, so they were
never built (#10):

- a **status summary**, counting a set of agents by state, shown once for the
  whole dashboard and again beside each workspace's name
  (`StatusSummaryView`, placed at `DashboardView.swift:58` and `:126`);
- a **quick prompt** field on every agent card, sending a prompt to that
  agent without opening its workspace window (`QuickPromptField`, placed in
  `AgentCardView`, sent through `AgentManager.injectText`).

The Swift files were removed in `d0a84963`; they are read from
`d0a84963^`.

The issue asks first whether they are wanted. Both are: the grid answers
"how is this agent" but never "how is this workspace", and the Command Center
exists to oversee agents without opening their windows, so a card that can be
answered without leaving it serves that purpose. Neither needs I/O on the
render path, and the quick prompt reuses the one delivery path every other
prompt already uses.

## What Changes

**Status summary** — carried over, extended by one state.

- A count per state, in the Swift order: awaiting input, working, idle,
  error. A state with no agents is left out. Shell agents are not counted,
  as in Swift.
- New: a **not running** count. The Rust card already shows a fifth state
  for an agent with no session (`working_indicator::state`'s `Off`); a
  summary that filed those under their stale state would disagree with the
  cards beneath it. Swift had no such state.
- Shown in the dashboard header for every agent on it, and beside each
  workspace section's name for that section's agents, in both the workspace
  dashboard and the Command Center.

**Quick prompt** — carried over, with three deliberate differences.

- Enter sends; Cmd+Enter sends and opens the agent's workspace window with
  the agent selected (Swift's "Send & Go"). Empty or whitespace-only text
  sends nothing. The field clears after sending.
- **Busy agents queue instead of disabling.** Swift disabled the field while
  the agent was working. Knot has a prompt queue
  (`queued-message-management`), and the quick prompt goes through
  `deliver_panel_prompt` like the composer, so a busy agent's prompt queues
  under the same rules. The field says it will queue rather than refusing.
- **Panel agents only.** Swift typed into the agent's terminal. Shell agents
  have no conversation and no `deliver_panel_prompt` path, and typing a
  command into a shell from a card is a different and riskier feature; the
  field is not drawn on a shell agent's card.
- **Not running means no field input.** An agent without a session (never
  started, deactivated, or its workspace window closed) shows the field
  disabled, saying it is not running. Swift had no such case.

**Dropped**: nothing. Swift's hover scale and shadow are styling and stay
with the existing card's own look.

## Impact

- `dashboard` spec: two ADDED requirements, and the "Agent card grid"
  requirement MODIFIED to include the quick prompt.
- `knot`: `dashboard.rs` (summary and field), `command_center.rs` and the
  workspace dashboard (routing), `l10n` keys. Cards grow by the field's
  height (`CARD_HEIGHT`).
- No new dependency, no settings change, no change to how prompts are
  queued or delivered.

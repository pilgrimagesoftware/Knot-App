## Context

`agent-hooks`'s "Autopilot classification trigger" requirement already
names the trigger condition; this change implements it plus everything
downstream. The Swift reference's `AutopilotService` is one actor doing
everything - classify, call the LLM, dispatch, touch `AgentManager`
directly. The Rust port has no such single owner: hook handling
(`knot-mcp-tools`) runs on the MCP server's own tokio runtime, off any
GPUI window; the agent-store mutation and the dialog both need a window.

## Decisions

### Split: pure crate + hook-side trigger + window-side effect

`knot-autopilot` knows nothing about agent ids, hooks, or GPUI - it takes a
message and settings, returns an `Effect`. `knot-mcp-tools::autopilot`
reads the hook payload, decides whether to classify at all, and turns a
landed `Effect` into an agent-store write plus a queued
`knot_mcp_tools::AutopilotEffect` (dropping `None`, which nothing
downstream needs to see). `crates/knot::workspace_window::autopilot`
drains that queue and does the one thing each effect needs a window for:
nothing for Mark (the state write and awaiting-input push already
happened), the decision dialog for Ask, `deliver_prompt` for InjectText.

This mirrors the awaiting-input/activation queue pattern already
established in `app_support.rs` exactly, rather than inventing a different
cross-thread handoff for the third queue.

### Mark and Ask both go through the existing awaiting-input queue

The Swift reference's `markInput` (called for both `.mark` and `.ask`) sets
status to `.input` and raises a generic notification. Rather than growing
a second notification path, `maybe_trigger_autopilot` pushes `(id, None)`
onto the same `AwaitingInputQueue` an explicit `status: "input"` hook
already uses - `notification_body` already treats `None`/empty as the
generic awaiting-input copy, so no new string plumbing was needed for that
half.

### Classification failure is silent; custom-action failure falls back to Mark

Ported as-is from the Swift reference's two different `catch` blocks:
`classify`'s failure returns `.completed`, which `analyze` then treats as
"nothing to do" - a transient classifier outage should not start paging on
every idle turn. `analyzeCustom`'s failure explicitly calls `markInput`,
because a user who configured a custom prompt has opted into *something*
happening and silence there would look broken rather than quiet.

### Blocking HTTP via `ureq`, not an async client

Classification is one request per idle turn, not a hot path; `ureq` is
already a workspace dependency (used nowhere yet) and avoids pulling in an
async HTTP stack for three call sites. The async boundary is
`tokio::task::spawn_blocking` inside the `tokio::spawn` that
`maybe_trigger_autopilot` already needs to keep `status()` non-blocking.

### Decision dialog uses `capped_scroll`, not a new scroll primitive

`crates/knot/src/capped_scroll.rs` already provides exactly "scrollable,
capped at a max height" - the Ask dialog's message pane uses it rather
than inventing a second scroll wrapper for one dialog.

## Risks

- The API key is still stored in plain text in settings (carried over from
  `2026-09-15-autopilot-settings-ui`'s Risks section - unchanged by this
  change).
- `ureq`'s blocking call inside `spawn_blocking` ties up one blocking-pool
  thread per in-flight classification; fine at the scale of "one agent
  going idle at a time" this app runs at.

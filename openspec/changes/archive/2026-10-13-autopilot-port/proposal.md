## Why

`2026-09-15-autopilot-settings-ui` ported Autopilot's settings surface
(enable toggle, provider/key, action picker, custom prompt) but explicitly
left the decision loop itself out of scope: nothing in the Rust port
detects when an agent needs input, classifies it, or takes an action.
Autopilot is dead UI today - the toggle persists, but idle agents are never
analyzed. The Swift reference's `AutopilotService` (tri-classification,
action matrix, custom-prompt path) and `InputDetectionService`'s hook
wiring are the behavioral spec for what this change ports.

## What Changes

- New `knot-autopilot` crate: the classification prompt and parser
  (completed/binary/open), the custom-action prompt prefix and
  empty-response sentinel, one `complete()` call per provider
  (OpenAI/Anthropic/Google) via `ureq`, and the pure action-matrix
  (`dispatch`/`dispatch_custom`) that turns a classification (or custom
  response) into an `Effect`: `None`, `Mark`, `Ask`, or `InjectText`.
- Wires the trigger `openspec/specs/agent-hooks/spec.md`'s "Autopilot
  classification trigger" requirement already documents: on a Claude `Stop`
  hook (transcript extraction) or a Codex turn-complete event
  (`payload["last-assistant-message"]`), when autopilot is enabled and an
  API key is configured, `knot-mcp-tools` spawns classification off the
  hook-handling thread and queues the resulting effect for whichever
  workspace window owns the agent.
- Mark reuses the existing awaiting-input queue and notification path
  verbatim (same queue entry an explicit `status: "input"` hook would
  push). Ask opens a new `AutopilotDecisionSheet` dialog (message, Dismiss/
  Switch/[Continue]). Auto-continue injects `"yes, continue"` for a binary
  classification only, falling back to Mark for an open one. Custom skips
  classification, sends the user's own prompt, and injects the response
  (or does nothing for an empty/`EMPTY` reply), falling back to Mark on an
  LLM error.
- New `autopilot` capability spec (`openspec/specs/autopilot/spec.md`)
  documenting the classification contract and action matrix; `agent-hooks`
  is unchanged - its trigger requirement already describes the target
  state this change implements.

## Non-Goals

- No streaming/incremental classification - one request, one response.
- No per-agent autopilot override - the setting is global, matching the
  Swift reference and the existing settings UI.

## Capabilities

### Added Capabilities

- `autopilot`: classification, the action matrix, and the custom action.

## Impact

- New crate `crates/knot-autopilot`.
- `crates/knot-mcp-tools/src/autopilot.rs` (new): the trigger and the
  agent-store/queue plumbing.
- `crates/knot/src/app_support.rs`, `app_bootstrap.rs`: a third queue
  (`McpQueues`, replacing three loose arguments to `start_mcp_server`).
- `crates/knot/src/workspace_window/autopilot.rs` (new): drains the queue.
- `crates/knot/src/autopilot_decision_sheet.rs` (new), `window_options.rs`:
  the Ask dialog.
- `crates/knot/src/settings_window/panes/autopilot.rs`: `ai_model_for` now
  reads from `knot-autopilot` instead of duplicating the mapping.
- `crates/knot-core/locales/en.yml`: the dialog's strings.

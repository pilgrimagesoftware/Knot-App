## 1. `knot-autopilot` crate

- [ ] 1.1 Classification prompt/parsing (`classify.rs`): `Classification`
      enum, `parse_classification`, `is_empty_response`, the
      classification and custom-prompt-prefix constants ported verbatim
      from the Swift reference.
- [ ] 1.2 LLM calls (`llm.rs`): one `complete()` covering OpenAI,
      Anthropic, and Google via `ureq`, plus the shared `ai_model_for`
      mapping the settings pane now reads instead of duplicating.
- [ ] 1.3 Action matrix (`dispatch.rs`): `Effect` enum, `dispatch` (plain
      classification) and `dispatch_custom` (custom-action response), pure
      and unit-tested with no mock LLM.
- [ ] 1.4 Orchestration (`lib.rs`): `analyze()`, branching to the custom
      path before classification, swallowing classify failures to `None`
      and custom-action failures to `Mark`.

## 2. Hook-side trigger

- [ ] 2.1 `knot-mcp-tools::autopilot`: `last_message_for_trigger` (Claude
      transcript on `Stop`, Codex `payload["last-assistant-message"]`),
      `maybe_trigger_autopilot` spawning classification off the current
      tokio runtime when autopilot is configured, pushing the landed
      effect onto a new queue and (for Mark/Ask) onto the existing
      awaiting-input queue.
- [ ] 2.2 Wired into `McpToolCatalog::status()` on every `Idle`
      transition.

## 3. Window-side effect

- [ ] 3.1 `McpQueues` struct bundling the three cross-thread queues
      `start_mcp_server` takes, replacing three loose arguments.
- [ ] 3.2 `workspace_window::autopilot::drain_autopilot_effects`, polled
      from `repaint_poll_tick`: no-op for Mark, opens the decision dialog
      for Ask, `deliver_prompt`s for InjectText.
- [ ] 3.3 `AutopilotDecisionSheet`: message (via `capped_scroll`),
      Dismiss/Switch always, Continue only for a binary classification.

## 4. Final verification

- [ ] 4.1 `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, `cargo build --workspace`, `make
      size-check`, `make declared-check`, and `make fmt` all pass clean.
- [ ] 4.2 Manually exercise Mark/Ask/Continue/Custom against a live Claude
      Code agent and a real API key.

# Proposal

## Why

Claude Code can ask the user a question with predetermined answers: one or
more questions, each single- or multi-select, each with an optional "Other"
free-text answer (the AskUserQuestion tool). Knot has no UI for it (#430).

Knot never sees it at all. `claude-agent-acp` only offers AskUserQuestion to
the model when the client advertises form elicitation in `initialize`
(`clientCapabilities.elicitation.form`); otherwise it puts the tool in
`disallowedTools`. Knot's `initialize` sends no client capabilities, so every
Knot session runs without the tool, and the model falls back to asking in
plain text. A live probe against the installed adapter (0.107.3) confirmed
both halves: advertising the capability gets an `elicitation/create` request
and a working answer round trip; not advertising it gets "no such tool".

The same capability also unlocks MCP servers' own input requests and Claude's
refusal-fallback consent prompt, which reach the client by the same request
and today are declined (`-32601`) without the user seeing them.

## What Changes

- **Advertise form elicitation.** `initialize` carries
  `clientCapabilities.elicitation.form`. URL-mode elicitation is not
  advertised.
- **Handle `elicitation/create` (form mode)** in the ACP client like a
  permission request: surfaced as a distinct event, held until answered, and
  answered `cancel` on Stop and on session end. The form schema is parsed at
  the boundary into typed fields (single-select, multi-select, text, number,
  boolean) in the order the agent sent them; a form Knot cannot render is
  answered with an error at once, which the adapter turns into "could not
  present the question" for the model. Field order relies on `preserve_order`, which `knot-acp`
  now declares itself.
- **Honour `$/cancel_request`.** When the agent withdraws a pending question
  or permission request, its card goes away. Knot ignores this notification
  today, so a withdrawn permission card stays up until the turn ends.
- **Question card** in the conversation, at the position the permission card
  uses: each question's header and text, its options as a radio or checkbox
  list with their descriptions, and its "Other" field below them. **Submit**
  sends the answers; **Skip** declines (the model is told the user skipped).
  Required fields block Submit until filled.
- **Keyboard**: the card takes focus when it appears on the visible panel;
  arrows move between options, Tab between fields, Space picks or toggles,
  Return picks and moves to the next question, ⌘Return submits, Escape skips.
  The composer keeps any text typed in it.
- **A pending question counts as a pending permission** everywhere Knot
  treats one specially: the composer queues instead of sending, steering
  waits, the agent shows **Awaiting input** with the question as the detail
  (and the desktop notification that comes with it), a scheduled run is
  skipped, and idle shutdown leaves the agent alone.
- **No timeout**, as for permissions: the question waits for the user, Stop,
  or the agent withdrawing it.

Agents that never send `elicitation/create` are unaffected: Claude Code
before form support, and any adapter that keeps asking in text. Terminal-mode
agents keep their own TUI prompt.

## Impact

- `knot-acp`: `InitializeParams` gains client capabilities; new
  `protocol/elicitation.rs` (typed form, ordered parse, response); inbound
  dispatch for `elicitation/create` and `$/cancel_request`; one pending map
  for both request kinds.
- `knot`: `PanelState`'s pending permission becomes a pending-input queue of
  permission or question; the question card (render, state, keyboard
  actions); the gating sites (`pane.rs`, `prompt.rs`, `steer.rs`,
  `panel_activity.rs`, `scheduling.rs`, `idle.rs`, `subagent_feed.rs`).
- `knot-core`: a `QuestionPending` schedule skip reason and catalog strings.
- Specs: `acp-client`, `acp-panel-ui`, `session-scheduling`.
- Docs: a page section on answering an agent's question.
- No new dependencies. `knot-acp` declares `serde_json`'s `preserve_order`
  feature, which the app build already has through gpui.

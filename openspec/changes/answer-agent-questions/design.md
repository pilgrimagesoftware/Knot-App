# Design

## Wire path (verified)

Probe: a minimal JSON-RPC client driving `claude-agent-acp` 0.107.3 (the
version installed by `npm install -g @agentclientprotocol/claude-agent-acp`;
npm's latest is 0.108.0) with a prompt asking Claude to use AskUserQuestion
for one single-select and one multi-select question. Run twice, once
advertising `clientCapabilities.elicitation.form = {}` and once advertising
nothing.

Without the capability the adapter adds AskUserQuestion to
`disallowedTools` (`v1/acp-agent.js`: `elicitationSupport.form ? [] :
["AskUserQuestion"]`), and the model answered "NO_ASK_TOOL".

With it, in order:

1. `session/update` `tool_call`: `toolCallId: "toolu_019a…"`, title "Asking
   for your input", kind `other`, status `pending`.
2. `session/update` `tool_call_update` with `rawInput.questions` (question,
   header, multiSelect, options with label and description).
3. Request from agent to client:

```json
{"jsonrpc":"2.0","id":0,"method":"elicitation/create","params":{
  "mode":"form","sessionId":"9ab6…","toolCallId":"toolu_019a…",
  "message":"Please answer the following questions.",
  "requestedSchema":{"type":"object","properties":{
    "question_0":{"type":"string","title":"Color","description":"Which color?",
      "oneOf":[{"const":"Red","title":"Red","description":"Pick red"}, …]},
    "question_0_custom":{"type":"string","title":"Other",
      "description":"Type your own answer, or add a note to the option you chose above (optional)."},
    "question_1":{"type":"array","title":"Fruits","description":"Which fruits?",
      "items":{"anyOf":[{"const":"Apple","title":"Apple","description":"Include apple"}, …]}},
    "question_1_custom":{"type":"string","title":"Other",
      "description":"Type your own answer to add to your selection above (optional)."}}}}}
```

4. Reply: `{"action":"accept","content":{"question_0":"Red","question_1":["Apple"]}}`.
5. `tool_call_update` status `completed`, rawOutput `Your questions have
   been answered: "Which color?"="Red", "Which fruits?"="Apple". …`; the model
   then repeated the answers.

How the adapter reads each reply (`dist/elicitation.js`,
`answerAskUserQuestion`):

| Reply | Model sees |
| --- | --- |
| `accept` with content | the answers; a blank field is "skipped"; `question_<n>_custom` is the answer when nothing was picked, an addition for a multi-select, a note beside a single-select pick |
| `decline` | every question skipped; the turn continues |
| `cancel` (or unknown action) | the tool use is aborted |
| JSON-RPC error | the tool is denied: "Could not present the question to the user." |

When Claude Code withdraws the request (Stop, `session/close`, the tool
aborted), the adapter sends `$/cancel_request` `{requestId}` and stops
waiting; a later reply is ignored. ACP's v1 cancellation answers a cancelled
request with error `-32800`.

Other adapters (string evidence only, not probed, not blocking):
`codex-acp`, `opencode` and `gemini` binaries all contain `elicitation/create`
(codex-acp also `request_user_input`, gemini an `ask_user` tool). If they send
form elicitations they get the same card with no extra work; if they don't,
nothing changes.

## What else arrives once `form` is advertised

The adapter forwards three things as form elicitations, so the card is a
renderer for the schema subset, not an AskUserQuestion special case:

- AskUserQuestion (above), with a `toolCallId`.
- MCP servers' `elicitation/create` (the SDK's `onElicitation`): flat forms
  of string, number, integer, boolean and enum fields, with `required`; no
  `toolCallId`.
- Claude's refusal-fallback consent prompt: a single-select between
  retrying on the fallback model and keeping the refusal.

`url` mode is not advertised: it means opening a browser for an OAuth-style
flow, a separate feature.

## Parsing: typed and ordered

`protocol/elicitation.rs` turns `params` into:

```rust
pub struct QuestionRequest {
    pub rpc_id:       RequestId,
    pub tool_call_id: Option<String>,
    pub message:      String,
    pub fields:       Vec<FormField>, // in the agent's order
}
pub struct FormField { pub key: String, pub title: Option<String>,
                       pub description: Option<String>, pub required: bool,
                       pub kind: FieldKind }
pub enum FieldKind {
    Single(Vec<Choice>),   // string + oneOf/enum
    Multi(Vec<Choice>),    // array, items anyOf/oneOf/enum
    Text { multiline: bool }, // plain string
    Number { integer: bool },
    Boolean,
}
pub struct Choice { pub value: String, pub title: String, pub description: Option<String> }
```

The workspace's `serde_json` has no `preserve_order`, so a `Map` comes back
sorted by key: `question_10` before `question_2`, and an MCP server's form in
alphabetical order. `properties` is deserialized through a small visitor into
`Vec<(String, Value)>`, keeping the wire order without turning the feature on
for every map in the workspace.

A form that does not fit (mode `url`, a nested object, an unknown field type)
is answered at once with `-32602` and no card; the adapter turns that into a
denied tool call (AskUserQuestion) or a `cancel` (MCP server).

The reply is built from typed answers (`QuestionAnswer::{Submitted(map),
Skipped, Cancelled}` mapping to `accept`/`decline`/`cancel`), with values
typed per field: a string for single-select and text, an array for
multi-select, a number, a boolean. Empty optional fields are left out.

## One pending map, one pending-input queue

`knot-acp`'s `PendingPermissions` becomes a map of either kind keyed by RPC
id. Stop (`session_cancel`) answers every entry `cancelled` (permission) or
`cancel` (question); session end does the same (permission keeps `Deny`, its
current answer). `$/cancel_request` removes the entry and answers `-32800`;
the existing resolved event (`PermissionResolved`, generalised to
`InputResolved { rpc_id }`) removes the card.

In `knot`, `PanelState.pending_permission` + `queued_permissions` become one
queue of `PendingInput::{Permission(PermissionRequest), Question(QuestionState)}`,
so the one on screen and the ones behind it keep arrival order across kinds.
Every site that read `pending_permission.is_some()` reads
`pending_input.is_some()`, which is how a question gets the same gating with
no per-site logic:

| Site | Effect |
| --- | --- |
| `pane.rs` | composer blocked state |
| `prompt.rs` | send queues; queued delivery waits |
| `steer.rs` | steering waits |
| `panel_activity.rs` | `AgentState::Input`, detail = the first question's text (or the form message) → sidebar status and the existing desktop notification |
| `scheduling.rs` | skip with `QuestionPending` |
| `idle.rs` | busy, never idle-reset |

Permission-only paths (`answer_selected_permission`, the allow/deny
actions) match the `Permission` variant only, so the permission shortcuts do
nothing while a question is on screen.

`subagent_feed.rs` ignores question events as it ignores permission events.

## The card

Rendered where the permission card is: after the last message, before the
working row. A question's own tool call is normally the last row above it,
so it reads in place. The tool-call row stays as it is (title "Asking for
your input"; after completion its output carries the answers), so the
transcript keeps the outcome after the card goes.

Layout per field: title (header) in emphasis, description (the question
text) below, then the control. Choices show their description as secondary
text. A text field whose key is `<key>_custom` directly after a choice field
renders indented under it with its own label ("Other"); that is the
adapter's convention for the per-question free-text box, and any other text
field renders on its own. Bottom row: **Submit** (primary) and **Skip**.

`QuestionState` holds the request and the user's in-progress answers. It
lives in `PanelState`, not in GPUI entities, so it survives re-renders and
panel switches; text inputs are created on attach and dropped with the card.

## Keyboard

The card takes focus when it is shown on the visible panel. The composer
keeps its text; focus returns to it when the card goes.

- Up/Down: previous/next choice, crossing into the next field.
- Tab/Shift-Tab: next/previous field.
- Space: pick (single) or toggle (multi).
- Return: on a single-select choice, pick it and move to the next question's
  first choice, past this question's Other field (a question is a choice
  field plus its `<key>_custom` field).
- Cmd-Return: Submit. Escape: Skip.

Actions are panel-scoped (`QuestionSubmit`, `QuestionSkip`,
`QuestionNext/Prev`), bound in the fixed keymap like the permission actions,
with the keystroke shown on the Submit and Skip buttons as the permission
card does.

## Cancel and timeout

No timeout: permissions have none, and an unanswered question costs nothing
while it waits. The ways out: Submit, Skip, Stop (answers `cancel`; the
adapter aborts the tool), the agent withdrawing it (`$/cancel_request`), the
session ending.

## Testing

- `knot-acp`: parse the captured AskUserQuestion request (fixture from the
  probe), an MCP-style form with every field type, and a 12-field form
  (order); reject url mode and a nested object with `-32602`; reply
  serialization per answer; Stop and session end answer `cancel`;
  `$/cancel_request` answers `-32800` and emits the resolved event, for a
  question and for a permission; `initialize` carries the capability.
- `knot` state: queue order across kinds; each gating site with a question
  pending; permission actions ignore a question.
- Card: field-state tests for pick/toggle/Other/required, keyboard
  navigation as a pure function over the field list.

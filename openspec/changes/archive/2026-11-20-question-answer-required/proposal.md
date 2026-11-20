## Why

A question card let Submit go with nothing picked. `claude-agent-acp` marks
none of AskUserQuestion's fields required (so a client can skip), so the empty
`accept` was valid by the schema, and the model was told "The user did not
answer the questions" (#465). The card's keyboard focus was a filled row,
which made the first option look picked when it was not.

## What Changes

- Submit is unavailable until every question with an "Other" field has a
  pick or non-blank Other text. Skip still declines.
- The focused choice is outlined rather than filled, so focus no longer looks
  like a selection.
- `knot-acp` logs each question request: received (with its field count),
  answered (with how many fields were filled), declined, cancelled, withdrawn
  or refused. Question and answer text never reach the log.

## Capabilities

### Modified Capabilities

- `acp-panel-ui`: "Submitting or skipping a question card" and "Question card
  keys".

## Impact

- `crates/knot/src/panel_state/question.rs`, `panel_view/question_card.rs`
- `crates/knot-acp/src/client/inbound.rs`, `transport/mod.rs`

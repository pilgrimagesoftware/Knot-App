# Proposal

## Why

A Markdown table in a response could break a word mid-word ("Observabili" /
"ty") in a column of short labels while the column beside it had plenty of
room (#400). The table layout gave each column a width in proportion to its
longest cell's character count, so a column of single words next to a column
of prose got a few percent of the table, less than its own widest word.

## What Changes

- Table columns are sized from their content, like CSS auto table layout.
  Each column is measured for its min-content width (the widest piece of text
  that cannot wrap) and its max-content width (its widest line). Every column
  gets at least its min-content width, and the room beyond that goes to the
  columns that would be wider in proportion to how much wider.
- Cell text wraps at whitespace and after a hyphen joining two words. A word
  breaks only when the columns' min-content widths do not fit the table's
  width.
- Every row resolves to the same column widths, so the columns stay aligned.

## Impact

- `gpui-base` fork: one commit on a new branch, `knot/table-columns-0.7.0`,
  cut from `knot/link-hover-0.7.0`, changes the default table layout
  (`text/table_columns.rs`, `text/node.rs`). Knot's `[patch.crates-io]` points
  at it. `docs/adr/0006-gpui-base-table-columns.md` records the decision.
- `knot`: no code change; every Markdown surface uses the fixed layout.
- `acp-panel-ui`: a requirement for table column sizing and wrapping.

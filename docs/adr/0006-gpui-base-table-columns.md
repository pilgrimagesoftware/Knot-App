---
id: ADR-0006
title: Size Markdown table columns in the gpui-base fork
status: proposed
date: 2026-11-12
tags: [ui, dependencies, gpui-kit, markdown]
---

# ADR-0006: Size Markdown table columns in the gpui-base fork

## Context

Issue #400: in a response's Markdown table, a column of short labels broke
"Observability" mid-word while the column beside it had plenty of room.

Tables are drawn by gpui-kit's `TextView`. Its default (wrapping) table layout
gave each cell `w(relative(n))`, where `n` is the column's longest cell in
characters, capped at 150. A column of single words beside a column of prose
therefore got about 13/163 of the table, narrower than its own widest word,
and the prose column kept room it did not need.

Every table row is its own flex row, so consistent column widths have to come
from the table renderer, which sees all rows. Knot can only pass a
`table_cell` style refinement, which is the same for every column and knows
nothing about content. The layout cannot be fixed from Knot.

`TextView` also has an opt-in scroll layout (`style.table` with overflow-x:
scroll) that measures max-content widths. Its shrink floors are 160-480px per
column, so in a panel a table with a prose column scrolls horizontally rather
than wrapping.

Knot already consumes a forked `gpui-base` (ADR-0003, ADR-0004).

## Decision

Add one commit to the fork, on a new branch `knot/table-columns-0.7.0` cut
from `knot/link-hover-0.7.0`. Knot's `[patch.crates-io]` points at the new
branch.

The commit sizes the default table layout like CSS auto table layout:

- Each column is measured across all rows, from shaped text, for its
  min-content width (the widest piece of a line that cannot wrap; lines wrap
  at whitespace and after a hyphen joining two words) and its max-content
  width (its widest line). An inline object cannot wrap, so both are its
  intrinsic width.
- A cell's `flex-basis` is its column's min-content width, its `flex-grow` is
  `max - min` (or `max` when no column has slack), and it shrinks down to
  `min(48px, min-content)`. These depend only on the column, so every row
  resolves to the same widths.
- So each column gets its min-content width plus a share of the remaining
  room in proportion to how much wider it wants to be. A word breaks only when
  the min-content widths do not fit.

The pure parts (wrap points, flex settings) live in a new
`text/table_columns.rs` with unit tests. The measurement replaces
`measure_table_columns` in `text/node.rs`, and the scroll layout keeps reading
the max-content widths it read before. Wrap-layout cells get a
`debug_selector` (compiled out of release builds) so Knot's layout tests can
measure them.

## Options considered

- **Fix the wrapping layout in the fork** (chosen). It is the layout every
  Knot Markdown surface already uses, and the fix is one commit in the table
  code, not in the text engine.
- **Opt Knot into the scroll layout.** No fork change, and its measured widths
  would stop the label column from breaking. But its 160-480px floors make a
  table with a prose column scroll sideways in a normal-width panel, which
  trades one readability problem for another.
- **Equal or fixed column widths.** Simple, but the issue is exactly a column
  being given a width unrelated to its content.
- **Knot only, through `table_cell` styling.** Not possible: one refinement
  applies to every column, and each row lays out on its own.

## Consequences

- The fork now carries seven commits. Its exit condition gains a third item:
  an upstream table layout that sizes columns from content. Until all three
  land, every gpui-kit bump rebases seven commits.
- Measuring min-content shapes every unbreakable piece of every cell once per
  render, as the scroll layout already did for whole lines. Tables in
  responses are small; a very large table would show this first.
- Width resolution relies on each cell's flex settings being the only input,
  which holds because cells set an explicit `min-width` and
  `overflow: hidden`. A later change that lets content set a cell's minimum
  size would let rows disagree.
- `knot/link-hover-0.7.0` and `knot/auto-grow-0.7.0` are kept so that
  reverting this still builds. Do not delete or force-push any of the
  branches: CI resolves them.
- The commit is a candidate to propose upstream (longbridge/gpui-kit).

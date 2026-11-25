# Proposal

## Why

Header text in a rendered agent response cannot be selected (#491). A drag
that starts or ends on a header selects nothing, so a header cannot be copied
on its own or as the start of a copy.

The cause is how headers get the title font. Knot claims every heading node
through gpui-base's block parser and draws it as a plain `div` of text, because
gpui-kit 0.6.4 had no per-heading style. gpui-base leaves a claimed node out
of selection, so the title face cost selection. The same claim flattened a
header's inline marks to plain words, which `acp-panel-ui` recorded as an
accepted loss.

gpui-base (0.7, the fork Knot builds against) now applies a per-level heading
`StyleRefinement` (`TextViewStyle::with_heading`) over its own heading
rendering. The archived `swap-ui-title-font-roles` design named this as the way
to delete the block parser and renderer.

## What Changes

- Markdown headers are drawn by gpui-base's own heading branch, styled with the
  title family and the level's size and weight through `with_heading`. The
  custom heading node, its parser and its renderer are removed.
- Header text is selectable and copyable like a paragraph, including a
  selection that starts or ends on a header.
- A header's inline marks render as they do in body text, in the title family:
  `## A **bold** word` shows "bold" as a bold run. This reverses the accepted
  loss in `acp-panel-ui`. It matches the Swift app, whose Markdown view renders
  header marks and lets all text be selected.
- No change to the faces, sizes or weights headers already draw with.

### Non-goals

- The `cmd-`/keyboard copy path; copying already works for anything selected.
- Any change to body text, code blocks or tables.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `acp-panel-ui`: "Rendered Markdown separates body from headers by font" no
  longer requires flattened header marks. A new requirement makes header text
  selectable like body text.

## Impact

- `crates/knot/src/markdown_view.rs`: the heading parser and renderer give way
  to a `MarkdownStyle` built per render from the theme, with a heading
  refinement. Both Markdown surfaces take it.
- `panel_view::PanelStyle` carries the `MarkdownStyle` instead of the title
  family alone.
- No new dependencies.

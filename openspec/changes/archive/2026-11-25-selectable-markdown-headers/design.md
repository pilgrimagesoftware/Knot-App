# Design

## Context

`markdown_view` builds both Markdown surfaces. It claimed headings with
`markdown_block_parser` and drew them with `markdown_block_renderer` as a
`div` carrying the flattened text. gpui-base counts a claimed (custom) block
only in a select-all copy: a drag's per-block walk asks it for selected text
and gets nothing, and it has no text layout to start or end a drag on.

## Decisions

### Style headings through `TextViewStyle::with_heading`

gpui-base's heading branch draws the heading's inline children through the same
selectable paragraph renderer as body text, then applies
`style.heading(level)`. A refinement setting the title family, the level's size
against the configured body size, and the level's weight reproduces today's
header exactly, minus the claim.

### Rebuild the themed style per render

A view with no style gets the one gpui-component installs for every text view,
derived from the theme. Passing a style replaces it rather than refining it,
and the installed one cannot be read back. `MarkdownStyle::new(theme, ..)`
therefore rebuilds gpui-component 0.7.0's `base_text_view_style` mapping from
`cx.theme()` and adds the heading refinement. Built per render, it tracks the
system appearance as the installed style does.

- Alternative: read the installed style back. Rejected for now: it needs a
  getter on `TextViewDefaults` in the gpui-base fork. It would remove the
  duplicated mapping and is worth doing upstream.

### `gpui-base`'s `TextView`, not gpui-component's wrapper

The wrapper's style type has a heading size hook but no family. With no style
and no motion set, the wrapper adds nothing to the inner view, so using the
inner view directly changes nothing else.

## Risks / Trade-offs

- **[Duplicated theme mapping]** A gpui-kit bump that changes
  `base_text_view_style` leaves Markdown colors disagreeing with other text
  views. The module doc names the check, beside the heading table it already
  carries.
- **[Header marks now render]** A header with inline code or a link now shows
  them styled. This is the behaviour the flattening rule was compensating
  for, not a new risk.

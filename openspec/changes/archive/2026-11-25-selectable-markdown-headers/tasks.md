# Tasks

## 1. Headers through gpui-base's heading branch

- [x] 1.1 Replace the heading block parser and renderer in `markdown_view.rs` with a `MarkdownStyle` carrying a `with_heading` refinement (title family, level size against the body size, level weight) on the themed style. Verify: `tests/markdown_view.rs` checks the refinement's family, size and weight.
- [x] 1.2 Pass the style from both surfaces: `PanelStyle` for the panel, the Markdown pane directly. Verify: `cargo check -p knot --all-targets`.

## 2. Selection

- [x] 2.1 Window tests that drag across rendered Markdown: a drag starting on a header, every header level, a drag across a header, and every other block kind. Verify: the tests that start on a header fail with the old claimed heading node (checked by restoring it locally) and pass now.

## 3. Gate

- [x] 3.1 `make fmt-check size-check declared-check lint test` and `openspec validate selectable-markdown-headers --strict` pass.

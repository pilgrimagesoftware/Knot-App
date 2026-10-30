---
id: ADR-0004
title: Publish the hovered link from the gpui-base fork
status: accepted
date: 2026-10-30
tags: [ui, dependencies, gpui-kit]
---

# ADR-0004: Publish the hovered link from the gpui-base fork

## Context

Issue #326 asks for a link's URL to show while the pointer rests on it, in
every Markdown surface: panel responses and the Markdown pane.

Links are drawn by gpui-kit's `TextView`. Each paragraph works out which of
its links the pointer is over, but only to set the link cursor. That answer,
and the link layout it comes from, are private. `TextView` has a public
`on_link_click` hook and no hover counterpart, and nothing public maps a
pointer position to a link. Knot cannot work it out from outside without
copying the text layout.

Knot already consumes a forked `gpui-base` (ADR-0003).

## Decision

Add two commits to the fork, on a new branch `knot/link-hover-0.7.0` cut from
`knot/auto-grow-0.7.0`. Knot's `[patch.crates-io]` points at the new branch.

The commit adds `text::hovered_link(window, cx) -> Option<HoveredLink>`
(URL, Markdown title, pointer position) to `gpui-base`:

- A paragraph that has links records the one under the pointer on mouse move,
  and again at paint so that a link scrolled or streamed under a still pointer
  is found too.
- The slot remembers which paragraph wrote it (its inline state's address,
  stable while it is on screen). Only that paragraph clears it, so one
  paragraph losing the pointer cannot erase another's report.
- A change redraws the window: `refresh` from an event, or
  `request_animation_frame` when found while painting.
- A report lapses when its owner stops being painted with the pointer still on
  its link (a switched view, a row virtualized away, replaced text), since
  nothing it owns runs to clear it then. Each window frame gets an epoch, begun
  by the window's `TextSelectionLayer`, which every `Root` draws first. The
  owner's paint re-stamps its report, which redraws nothing. After a frame
  that ends with a report its owner did not re-stamp, the report is dropped
  and the window drawn once more. This was a second commit, `270145a6`, added
  after review.

Knot draws the label in a `Root` plugin (`crates/knot/src/link_tooltip.rs`),
so every window and every Markdown surface gets it without per-surface code.
The label has no mouse handlers and so no hitbox, which keeps it out of the
way of the hover that reveals a response's action bar (#313) and of the
link's own click.

## Options considered

- **A `hovered_link` global in the fork** (chosen). It is about 70 lines in
  one file plus two call sites, adds nothing to `TextView`'s builder, and is
  general enough to offer upstream.
- **An `on_link_hover` builder hook, mirroring `on_link_click`.** This is the
  more conventional API, but the click handler is threaded through about 25
  places (node, inline flow, inline objects, state). A second handler would
  double that diff and the cost of every later rebase.
- **A full tooltip inside `gpui-base`.** gpui's tooltip timing is private to
  `div` (`register_tooltip_mouse_handlers`), so this would mean copying it.
- **Claim link nodes with a Markdown inline plugin and draw them in Knot.** An
  inline element is atomic: a long link would stop wrapping and stop taking
  part in text selection.
- **Knot only, with no fork change.** Not possible: nothing public says where
  a link is.

## Consequences

- The fork now carries six commits, and its exit condition is two things: an
  upstream release with the relaxed `auto_grow` bound (ADR-0003) and a
  link-hover hook. Until both land, every gpui-kit bump rebases six commits.
- The lapse rule assumes a paragraph on screen paints every frame. A view
  drawn through gpui's view caching (`AnyView::cached`) can skip painting while
  still on screen, so its link would lapse under a still pointer. Knot caches
  no views today; anything that starts doing so should check that the URL
  label survives a still hover.
- `knot/auto-grow-0.7.0` is kept so that reverting #326 still builds. Do not
  delete or force-push either branch: CI resolves them.
- The commit is a candidate to propose upstream (longbridge/gpui-kit). If
  upstream takes a different shape, such as a builder hook, only
  `link_tooltip.rs` needs to follow it.

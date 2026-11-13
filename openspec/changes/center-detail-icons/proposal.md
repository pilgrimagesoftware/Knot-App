# Proposal

## Why

The icons on an agent row's detail lines sit at the top of their line, so
each one looks offset above the text it labels (#403). The row aligns each
line's icon to the top so that a wrapped persona keeps its icon beside its
first line. But the icon is shorter than a text line, so top-aligning it
leaves it high.

## What Changes

- Each detail line's icon is centred vertically on the first line of its
  text, at both line sizes.
- A line that wraps still keeps its icon beside its first line, centred on
  that line, not on the whole block.

## Impact

- `knot`: `detail_line` (`workspace_window/chrome.rs`) puts the icon in a
  box one text line tall, styled like the line's text, and centres the icon
  in that box.
- `agent-list-ui`: the "labelled by an icon" requirement says how its icons
  sit against their lines, with a new scenario.

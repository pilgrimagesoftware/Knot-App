# Proposal

## Why

Claude Code's terminal shows how long a turn has run and how many tokens it has
produced - "(42s · 1.2k tokens · esc to interrupt)". The panel's working
indicator showed neither (#35).

## What Changes

- The working row shows the turn's elapsed time beside its animation, and once
  the agent has written something, an approximate token count:
  "42s · ~1.2k tokens".
- The time is exact, from when the prompt started the turn. The count is
  estimated from the text streamed this turn at four characters to a token:
  ACP's `usage_update` reports only the context window's occupancy, not a
  turn's output tokens, and thinking is not streamed to the panel, so a
  thinking-heavy turn reads low. The "~" says it is an estimate.

## Impact

- `knot`: `PanelState::turn_started_at` and `turn_text_chars`;
  `panel_state::turn_progress`; the Working row in `panel_view/render.rs`;
  `CHARS_PER_TOKEN` in `consts`.

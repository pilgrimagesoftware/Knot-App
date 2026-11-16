# Proposal

## Why

A Panel agent stopped by its session limit (#318) cannot answer until the limit
resets, but a prompt sent to it was delivered at once - and refused again (#368).
The user wants it to wait and go by itself when the limit lifts, with a way to
send it immediately anyway.

## What Changes

- A prompt to a session held by its usage limit is queued, not delivered.
- The limit's reset text - "5:20pm (America/Los_Angeles)" - is read as the
  next such time in that zone after the refusal. The queue pump, which runs
  every tick, delivers the head once that time has passed. A reset it cannot
  read (no time, a date, an unknown zone) holds until the user overrides.
- A queued row the limit is holding says so, and offers **Send now**: the entry
  moves to the head and goes on the next tick, past the limit.
- A delivered prompt clears the limit as before; if the provider refuses again,
  the new refusal sets a new limit and reset time.

## Impact

- New workspace dependency `jiff` (time zones from the system database), used by
  `knot-agents`' `SessionLimit::lifts_at`. The existing `time` crate has no zone
  database.
- `knot`: `PanelState::session_limit_lifts_at` and `held_by_session_limit`;
  `prompt_queue`'s `send_now` and the `held` input to `take_next`; the hold in
  `deliver_panel_prompt` and `drain_panel_prompt`; the queued row's label and
  Send now control.
- `SessionLimit`'s shape is unchanged, so its existing literals still build.

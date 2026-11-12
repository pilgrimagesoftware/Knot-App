# Proposal

## Why

Claude Code now names the usage limit it hit: "You've hit your 5-hour limit ·
resets 5:20pm (America/Los_Angeles)" where it used to say "session limit", and
also "weekly limit", "Opus limit", "Sonnet limit" and others. Both detectors
matched the old names literally: the Panel error path (`session-limit-status`)
and the terminal output scan (`terminal-session-limit`). So an agent stopped
by a limit stayed green and its held prompts never waited for the reset
(#397).

## What Changes

- A limit is recognised by the lead the Claude Agent SDK exports as stable in
  `USAGE_LIMIT_ERROR_PREFIXES` ("You've hit your", "You've reached your"),
  followed by a short name ending in "limit". A renamed or newly added limit
  is covered with no code change. The older "session limit" and "usage limit"
  wordings are still recognised.
- Fast mode's "You've hit your fast limit" uses the same lead but doesn't stop
  the session: the agent carries on at normal speed. It is not a session
  limit.
- The terminal scan still reads only the start of a line, so an agent that
  writes about a limit mid-sentence doesn't stop itself.

## Impact

- `knot-agents`: `SessionLimit::from_error` also accepts a named limit, and
  the new `SessionLimit::opens_with_named_limit` is the line-start check.
- `knot-terminal`: the session-limit scan uses that check. Its literal notice
  list keeps only the unnamed wrap-up ("Usage limit reached · wrapping up").
- Spec: `activity-detection` adds "A named usage limit is a session limit".
- Not covered: a weekly limit's reset is a date, which `lifts_at` doesn't read.
  The agent shows its limit, but held prompts aren't sent automatically when
  it lifts, as for any other reset Knot can't read.

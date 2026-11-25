---
id: ADR-0008
title: Evaluate cron schedules with `croner` over `jiff`
status: accepted
date: 2026-11-16
tags: [dependencies, scheduling, time]
---

# ADR-0008: Evaluate cron schedules with `croner` over `jiff`

## Context

Scheduled agent starts (#404, `openspec/changes/session-scheduling`) accept a
five-field cron expression as well as presets. Something has to turn an
expression and the current time into the next run, in the Mac's local time
zone, with defined behavior across daylight-saving changes: a skipped local
time runs at the first valid time after it, and a repeated one runs once.

The workspace already uses `jiff` for zoned time and does not use `chrono`.
The proposal first recommended a small evaluator of our own over `jiff`.
The maintainer approved the change on the condition that cron evaluation is
not hand-rolled.

## Decision

Use `croner` 4.x with `default-features = false, features = ["jiff"]`.

> **Update (2026-11-25):** now `croner` 5.x with
> `default-features = false, features = ["std", "jiff"]`, for `jiff` 1.0,
> which renamed `jiff::Zoned` to `jiff::ZonedDateTime`. `croner` 5 is
> `no_std` without its `std` feature, hence the addition. It also refuses
> patterns 4.x accepted on an unintended schedule (a name outside its own
> field, a bare `L` for a weekday) and reads `SUN-SUN` as Sundays only; a
> stored pattern it now refuses loads as unreadable and never runs. The DST
> tests below passed unchanged.

- `croner`'s default feature is `chrono`. Turning it off and enabling `jiff`
  gives `CronDateTime` for `jiff::Zoned`, so `Cron::find_next_occurrence`
  takes and returns the zoned times the rest of the change uses, and the
  workspace gains no `chrono`.
- The parser is built with seconds and years **disallowed**. `croner`
  accepts six- and seven-field patterns by default; the spec offers standard
  five-field cron, and a pattern with a seconds field would let an agent
  start every second.
- `croner` handles folds and gaps (`time/fold.rs`). The spec's DST rules are
  pinned by Knot's own tests at a spring-forward and a fall-back, so a
  `croner` upgrade that changes either fails a test, not a user's morning.
- Presets stay Knot's own human-friendly path. They are evaluated through the
  same `next_run`, which builds an equivalent cron pattern for each preset,
  so presets and cron share one evaluator and one set of DST rules.
- `Cron::describe()` gives an English description of an expression. The
  agent editor may show it as a preview, substituted into a catalog sentence.
  It is English-only, like the catalog today; a second locale would need
  `describe_lang` or Knot's own wording.

## Options considered

- **`croner` with `jiff`** (chosen). Maintained, standard Vixie and OCPS
  semantics including the day-of-month / day-of-week OR rule, explicit DST
  fold handling, and a `jiff` backend.
- **A hand-rolled evaluator over `jiff`** (rejected). The maintainer's rule
  for this change. Cron's corner cases (DOM/DOW OR, steps across ranges,
  DST) are where a small evaluator goes wrong quietly.
- **`jiff-cron`**: smaller and `jiff`-native, but younger and less used.
  `croner` brings the same `jiff` integration with more history behind it.
- **`cronexpr`**: a parser and evaluator, but with no `jiff` integration, so
  time-zone conversion would be ours to write.
- **`hron`**: a human-language schedule grammar. The presets already cover
  the human-friendly path, so a grammar is not needed.

## Consequences

- One new dependency, in `[workspace.dependencies]`, used by `knot-core`.
- `chrono` must stay out: a later change that enables `croner`'s default
  features would bring it back. The workspace entry says so beside it.
- The DST tests and a test that five-field-only parsing rejects six fields
  guard the behavior Knot relies on across `croner` upgrades.

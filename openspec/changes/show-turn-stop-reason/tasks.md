# Tasks

## 1. Keep the reason

- [x] 1.1 `StopReason` in `knot-acp`, parsed from the `session/prompt`
      response; `error` for a failed prompt; `Other` keeps an unnamed value
- [x] 1.2 Log the reason beside the response line, with no session id

## 2. Show it

- [x] 2.1 A notice for `max_tokens`, `max_turn_requests`, `refusal` and any
      unnamed reason; none for `end_turn`, `cancelled` or `error`
- [x] 2.2 Catalogue keys for each notice

## 3. Follow-ups

- [ ] 3.1 Stop parsing the spec-external `turn_end` session update, reworking
      the panel replay tests that drive a turn end through it

## 4. Tests

- [x] 4.1 `StopReason` round-trips every spec value, defaults a missing one to
      `end_turn`, and keeps an unknown one verbatim
- [x] 4.2 The fold adds one notice for an early stop and none for an ordinary
      end; an unknown reason survives substitution; every key resolves

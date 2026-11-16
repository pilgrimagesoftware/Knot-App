# Tasks

## 1. Recognise a limit by its stable lead

- [x] 1.1 Recognise "You've hit your" / "You've reached your" followed by a
      short name ending in "limit" in the error that answers a Panel prompt,
      keeping the older "session limit" and "usage limit" phrases
- [x] 1.2 Recognise the same at the start of a terminal agent's output line,
      keeping the wrap-up notice
- [x] 1.3 Exclude fast mode's "fast limit" cooldown, and a lead that names no
      limit ("your team's shared budget")

## 2. Tests

- [x] 2.1 Each limit name Claude prints today, with its reset, in a Panel error
      and in a decorated terminal line
- [x] 2.2 The fast-mode cooldown, a team budget and prose opening with the lead
      are not limits; a named limit mid-line is not a terminal notice
- [x] 2.3 Mutation-check: with the named-limit match turned off, the new
      tests and the existing hard-stop terminal tests fail

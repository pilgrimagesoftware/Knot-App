# Tasks

## 1. The characters belong to the agent type

- [x] 1.1 Add `Invocation { command, skill }` and an `invocation` column to
      the agent-type roster: `/` and `$` for Codex, `/` for every other type
      and for an unknown one
- [x] 1.2 Read it in the panel's composer styling and lookup when each is
      created, so nothing in the UI names a character

## 2. A skill character of its own

- [x] 2.1 Scan `$name` at a word boundary as a skill token for such an agent,
      leaving a price or a mid-word `$` as prose
- [x] 2.2 Open a skills-only lookup on it, keeping knot's commands and library
      prompts behind `/`
- [x] 2.3 Insert the completion behind the character the token was opened
      with

## 3. Tests

- [x] 3.1 Roster: only Codex splits its characters, and an unknown type is
      `/` for both
- [x] 3.2 Scan: `$skill` anywhere a word starts, a bare `$`, prose cases,
      `/` commands for Codex, and `$` as prose for an agent sharing `/`
- [x] 3.3 Lookup: a `$` token is a skill trigger, and Codex's skills move
      from `/` to `$`
- [x] 3.4 Mutation-check: with skill tokens never scanned, the skill-token
      tests fail

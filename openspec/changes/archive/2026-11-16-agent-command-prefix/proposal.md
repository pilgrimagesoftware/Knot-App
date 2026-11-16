# Proposal

## Why

The panel composer's command lookup always opened on `/` and offered commands
and skills together behind it (#272). That is Claude Code's convention, not
every harness's. Codex invokes a skill as `$name` and rejects `/name` by design
(openai/codex#11817), so for a Codex agent the lookup completed skills into a
spelling the agent refuses, and a `$skill` typed by hand was drawn as prose.

The other harnesses Knot runs were checked against their own documentation:

| Harness | Commands | Skills |
|---|---|---|
| Claude Code | `/` | `/name` |
| Codex | `/` | `$name` |
| Copilot CLI | `/` | `/name` |
| OpenCode | `/` | `/name` |
| Gemini CLI | `/` | none: the model activates skills itself |

## What Changes

- Each agent type states the characters its prompt opens a command and a skill
  with, as a column of the agent-type roster (`knot_core::agent_type`), not a
  literal in the UI. Every type is `/` for both except Codex, which is `/` for
  commands and `$` for skills. An unknown type is `/` for both.
- For an agent whose skills have a character of their own, the composer
  recognises that character as a second trigger. `$name` at the start of a
  word is a skill token, drawn as a command token is, and it opens a lookup
  that lists the agent's skills alone. `/` keeps knot's commands and library
  prompts.
- Insertion writes back the character the token was opened with.
- Nothing changes for an agent whose skills share `/`.

## Impact

- `knot-core`: `agent_type::Invocation`, the roster's `invocation` column,
  and `agent_type::invocation(id)`.
- `knot`:
  - `composer_scan` takes the agent's `Invocation` and adds
    `Construct::SkillToken`.
  - `panel_commands::Trigger` is `Command | Skill | Mention`, and
    `ActiveToken` carries the character that opened it.
  - `LookupRegistry::for_agent` / `skills_for` split commands from skills.
  - The panel's styling and lookup read the agent's `Invocation`.
- Spec: MODIFIED `panel-slash-commands` "Slash-triggered lookup" and
  `panel-rich-input` "Slash and mention tokens are styled".
- Not changed: knot's built-in commands come from its Claude plugin, and the
  skills listed are read from Claude's skill roots (`.claude/skills`). Codex's
  own roots (`.agents/skills`) are a separate gap.

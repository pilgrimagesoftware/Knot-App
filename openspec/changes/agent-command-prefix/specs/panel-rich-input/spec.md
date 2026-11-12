# Spec Delta

## MODIFIED Requirements

### Requirement: Slash and mention tokens are styled

A command token, a skill token, or an `@` reference in the buffer SHALL be
drawn distinctly from surrounding prose, so a token the lookup inserted is
visibly a token. The treatment SHALL apply to a token the user typed by hand
exactly as it applies to one the lookup inserted — the composer SHALL NOT track
provenance. The token characters SHALL be the agent's own: `/` for
commands, and a skill character where the agent type has one of its own.

A token SHALL be styled only where it is a token: a `/` mid-word, an `@` inside
an email address, or a skill character that does not open a word naming a
skill SHALL be left as prose.

#### Scenario: An inserted command is styled

- **WHEN** the lookup inserts `/review` into an empty buffer
- **THEN** that range is drawn with the token treatment

#### Scenario: A hand-typed token is styled the same

- **WHEN** the user types `/review` without opening the lookup
- **THEN** the range is drawn identically to an inserted one

#### Scenario: A slash inside a path is prose

- **WHEN** the buffer contains `crates/knot/src`
- **THEN** no part of it carries the token treatment

#### Scenario: An email address is prose

- **WHEN** the buffer contains `write to paul@example.com`
- **THEN** `@example.com` carries no mention treatment

#### Scenario: A Codex skill token is styled

- **WHEN** a Codex agent's buffer contains `use $prd for the plan`
- **THEN** `$prd` is drawn with the token treatment

#### Scenario: A price is prose

- **WHEN** a Codex agent's buffer contains `it costs $5`
- **THEN** no part of it carries the token treatment

#### Scenario: A lone skill character is prose

- **WHEN** a Codex agent's buffer contains `pay $ 5`
- **THEN** no part of it carries the token treatment, while a `$` typed at the
  end of a line is drawn as a token as it is being typed

#### Scenario: A dollar is prose where skills share the slash

- **WHEN** a Claude agent's buffer contains `$prd`
- **THEN** no part of it carries the token treatment

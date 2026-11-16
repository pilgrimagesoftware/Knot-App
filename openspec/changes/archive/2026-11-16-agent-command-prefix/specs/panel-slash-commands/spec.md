# Spec Delta

## MODIFIED Requirements

### Requirement: Slash-triggered lookup

When the user types the agent's command character - `/` for every agent type
Knot runs - at the head of a line in the panel prompt input, the system SHALL
show a lookup popup listing the commands the registry offers, and the agent's
skills too when they share that character. An agent type whose skills have a
character of their own SHALL have its skills offered behind that character
instead. Each list SHALL filter to entries whose token or description matches
the text typed after its character.

The command lookup SHALL NOT be the popup's only occupant. The popup is shared
with the `@` file lookup and with a skill character's lookup, and the token
under the caret decides which one is active; the command lookup SHALL yield
when the caret sits in an `@` token or a skill token, and SHALL resume when the
caret returns to a command token. The token the lookup inserts SHALL be one the
composer's styling recognises, written with the character that opened it, so
an inserted command or skill is drawn as a token rather than as prose.

#### Scenario: Slash opens the lookup
- **WHEN** the user types `/` at the start of a line in the prompt input of an
  agent whose skills share the command character
- **THEN** a popup listing the available commands and skills appears above
  the input

#### Scenario: Typing filters the lookup
- **WHEN** the user types more characters after the `/`
- **THEN** the popup shows only the entries whose token or description matches
  the typed text

#### Scenario: No matches dismisses the lookup
- **WHEN** the text after the `/` matches no entry
- **THEN** the popup shows no entries and is dismissed

#### Scenario: The caret moves into an `@` token
- **WHEN** a slash lookup is open and the caret moves into an `@` token
  elsewhere in the buffer
- **THEN** the popup shows the file lookup instead, and the slash list is not
  shown alongside it

#### Scenario: An inserted command is styled as a token
- **WHEN** the lookup inserts a command into the buffer
- **THEN** the inserted range is drawn with the composer's token treatment

#### Scenario: A Codex agent's skills are behind `$`
- **WHEN** the agent is a Codex agent and the user types `$re` at the start
  of a word
- **THEN** a popup lists the agent's skills matching `re`, and no command or
  library prompt

#### Scenario: A Codex agent's slash offers commands, not skills
- **WHEN** the agent is a Codex agent and the user types `/` at the start of
  a line
- **THEN** the popup lists knot's commands and library prompts, and none of
  the agent's skills, which Codex would reject as `/name`

#### Scenario: A completed skill keeps its character
- **WHEN** the user completes a skill from a lookup opened with `$`
- **THEN** the buffer reads `$` followed by the skill's name

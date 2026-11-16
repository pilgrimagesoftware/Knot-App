# Spec Delta

## MODIFIED Requirements

### Requirement: Every detail line on an agent row is labelled by an icon

An agent row's detail lines - the ones under the agent's name - SHALL each
carry a leading icon identifying what that line is: its agent type, its
persona, its status, and its folder.

Without one, a line is a bare string whose meaning has to be inferred from
its content, which fails exactly when it matters: an agent whose status text
happens to look like a path, or whose folder is named after a person, reads
as the wrong thing entirely.

The icons SHALL be drawn as icons rather than as characters inside the line's
text, so they share the lines' muted colour, size with their line, and sit in
one column down the row. Each icon SHALL be centred vertically on the first
line of its text, rather than aligned to the top of the line, where an icon
shorter than the text reads as sitting above it.

#### Scenario: A status line is identifiable as a status

- **WHEN** an agent row renders with a status
- **THEN** that line carries a leading icon marking it as the status

#### Scenario: A folder line is identifiable as a folder

- **WHEN** an agent row renders its folder
- **THEN** that line carries a leading icon marking it as a folder

#### Scenario: The icons line up

- **WHEN** an agent row renders with a type, a persona, a status and a folder
- **THEN** all four leading icons share the row's muted colour and align in
  one column

#### Scenario: An icon is centred on its line

- **WHEN** an agent row renders a detail line, at either line size
- **THEN** the line's icon is centred vertically on the first line of its
  text, neither above nor below it

#### Scenario: A line with nothing to show has no icon

- **WHEN** an agent has no persona
- **THEN** no persona line and no persona icon are drawn, rather than an icon
  beside an empty line

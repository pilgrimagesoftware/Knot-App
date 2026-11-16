# Spec Delta

## ADDED Requirements

### Requirement: The library windows have configurable shortcuts

The configurable shortcuts SHALL include **Open Personas** (default ⌥⇧⌘P),
**Open Prompts** (default ⌥⇧⌘I) and **Open Bench** (default ⌥⌘B), each opening
its library window or bringing it to the front, from any Knot window or none.
Each SHALL be a single-chord shortcut on the same terms as Toggle Artifacts,
stored when customized as `openPersonas`, `openPrompts` and `openBench`.

#### Scenario: The defaults open the windows

- **WHEN** the user has not customized them and presses ⌥⌘B
- **THEN** the Bench window opens, or comes to the front

#### Scenario: A customization is stored and honoured

- **WHEN** the user rebinds Open Bench to another free chord in the Keyboard
  pane
- **THEN** the new chord opens the Bench, ⌥⌘B no longer does, and the
  preferences document stores it under `openBench`

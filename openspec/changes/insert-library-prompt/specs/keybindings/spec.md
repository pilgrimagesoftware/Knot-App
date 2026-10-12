# Spec Delta

## ADDED Requirements

### Requirement: Inserting a prompt by shortcut

The application SHALL provide an Insert Prompt shortcut, ⌥⌘I by default. It
SHALL be configurable like the other single-chord shortcuts: it is listed on
the Keyboard tab and subject to the same conflict rules. When a panel
agent's composer has focus, the shortcut SHALL open that composer's Insert
Prompt list (`prompt-library` - The composer's Insert Prompt list). When
no panel composer has focus, it SHALL do nothing.

#### Scenario: Default opens the picker

- **WHEN** the user has never customized Insert Prompt and presses ⌥⌘I in a
  panel agent's composer
- **THEN** the Insert Prompt list opens for that composer

#### Scenario: Customized key

- **WHEN** the user rebinds Insert Prompt to ⌃⌘P
- **THEN** ⌃⌘P opens the list and ⌥⌘I no longer does

#### Scenario: No composer focused

- **WHEN** focus is on the agent list and the user presses ⌥⌘I
- **THEN** nothing happens

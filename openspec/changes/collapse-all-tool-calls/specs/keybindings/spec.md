## ADDED Requirements

### Requirement: Collapsing the tool calls by shortcut

⌥⌘C on macOS, and Ctrl+Alt+C elsewhere, SHALL collapse every tool call in the
selected agent's conversation, as View > Collapse All Tool Calls does. The
binding SHALL be fixed: it is not in the Keyboard settings pane, and a
customization that would take ⌥⌘C SHALL be refused as a conflict, like any
fixed shortcut.

#### Scenario: Pressing the key

- **WHEN** a conversation with open tool-call cards is on screen and the user
  presses ⌥⌘C
- **THEN** every card collapses to its header

#### Scenario: The key off macOS

- **WHEN** the same conversation is on screen on Linux or Windows and the user
  presses Ctrl+Alt+C
- **THEN** every card collapses to its header

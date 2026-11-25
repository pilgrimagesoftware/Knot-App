# Spec Delta

## MODIFIED Requirements

### Requirement: Agents menu

The menu bar SHALL carry an Agents menu offering the agent row context
menu's items, in its order and grouping, from one item set. On macOS a
Shift alternate SHALL be a native alternate of its primary; elsewhere only
the primary SHALL be shown. The menu SHALL act on the focused window's
selected agent, and an item SHALL do exactly what it does from that agent's
context menu, including any confirmation.

#### Scenario: The menu offers what the context menu offers

- **WHEN** the user opens the Agents menu with an agent selected
- **THEN** it lists the same items, in the same order and groups, as that
  agent's context menu

#### Scenario: Holding Shift shows the alternates

- **WHEN** the user opens the Agents menu on macOS with a running, non-shell
  agent selected and `restore-conversation-on-launch` enabled, and holds
  Shift
- **THEN** Restart Agent (⌘R) reads Restart with New Conversation (⇧⌘R)
  and Deactivate reads Deactivate with New Session, each in its primary's
  place

#### Scenario: No alternates off macOS

- **WHEN** the user opens the Agents menu on Windows or Linux
- **THEN** it shows Restart Agent and Deactivate, and no Restart with New
  Conversation or Deactivate with New Session item

#### Scenario: An item does the same thing from either menu

- **WHEN** the user selects Remove Agent from the Agents menu
- **THEN** the same confirmation appears, and confirming removes the same
  agent, as selecting Remove Agent from that agent's row

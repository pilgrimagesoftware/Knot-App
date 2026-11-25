# Spec Delta

## MODIFIED Requirements

### Requirement: Agents menu

The menu bar SHALL carry an Agents menu offering the same items as the agent
row's context menu, in the same order and grouping, produced from one item
set. A Shift alternate SHALL be its own item, directly below its primary.
The menu SHALL act on the agent selected in the focused workspace window's
sidebar, and an item SHALL do exactly what it does from that agent's
context menu, including any confirmation.

#### Scenario: The menu offers what the context menu offers

- **WHEN** the user opens the Agents menu with an agent selected
- **THEN** it lists the same items, in the same order and groups, as that
  agent's context menu, with each Shift alternate as its own item below its
  primary

#### Scenario: An alternate has its own item

- **WHEN** the user opens the Agents menu with a running, non-shell agent
  selected
- **THEN** Deactivate with New Session is listed directly below Deactivate,
  and Restart with New Conversation directly below Restart Agent

#### Scenario: An item does the same thing from either menu

- **WHEN** the user selects Remove Agent from the Agents menu
- **THEN** the same confirmation appears, and confirming removes the same
  agent, as selecting Remove Agent from that agent's row

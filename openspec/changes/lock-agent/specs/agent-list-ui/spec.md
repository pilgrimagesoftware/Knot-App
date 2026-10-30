# Spec Delta

## ADDED Requirements

### Requirement: Lock and unlock an agent

The agent row context menu SHALL offer Lock Agent for an unlocked agent and
Unlock Agent for a locked one, never both, and neither for a shell agent. The
menu bar's Agents menu SHALL carry both items, enabled as they apply to the
selected agent. The lock SHALL persist across a relaunch. A locked agent's row
SHALL show a Locked detail line with its icon, and the sidebar's Broadcast to
All Agents SHALL skip locked agents.

#### Scenario: Locking from the context menu

- **WHEN** the user selects Lock Agent on an agent's row
- **THEN** the agent is locked, its row shows Locked, and its context menu
  offers Unlock Agent instead

#### Scenario: Broadcast skips a locked agent

- **WHEN** the user broadcasts to all agents in a workspace with a locked agent
- **THEN** every agent but the locked one receives the text

### Requirement: Send prompt to marks a locked agent

The "Send prompt to" submenu SHALL list a locked agent and send to it when
chosen, since the user prompting an agent is not agent messaging. It SHALL mark
the agent locked, with a lock icon and in its label.

#### Scenario: Send prompt to marks a locked agent

- **WHEN** the user opens "Send prompt to" in a workspace with a locked agent
- **THEN** the locked agent is listed with a lock and marked locked, and
  choosing it sends the prompt

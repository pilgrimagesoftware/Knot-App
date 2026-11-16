# Spec Delta

## MODIFIED Requirements

### Requirement: A narrow sidebar shows a compact layout

Below a compact breakpoint of 160px the sidebar SHALL show a compact layout
rather than squeezing the full-width one, porting the Swift reference's
`isCompact` sidebar:

- An agent row SHALL show its avatar alone, centered, with the state dot
  overlaid on the avatar rather than in a column of its own. The name, agent
  type, companion marker, persona, activity line and folder SHALL be hidden, and
  the row's tooltip SHALL carry what they said: the agent's name, its state as
  its status dot's tooltip words it (for an agent that has a dot), and its
  activity line when it has one, so a row stays identifiable and what the
  agent is doing stays in reach (#253).
- The dashboard row SHALL show its icon alone, centered, with its label hidden.
- The sidebar's title bar SHALL hide the workspace name and keep the application
  icon. The workspace SHALL stay identifiable while the label is hidden: the
  window's OS title still carries the name, and the title bar SHALL offer the
  name as a tooltip, the same way a compact agent row does.
- The new-agent control SHALL show its icon alone, with its label hidden and its
  meaning available as a tooltip.

Everything that is not text SHALL behave as it does at full width: selection
highlighting, the dimming of an agent that is not running, the companion
indent, and every context menu and click target named elsewhere in this
capability.

At or above the breakpoint the sidebar SHALL show the full-width layout
unchanged.

#### Scenario: Crossing the breakpoint

- **WHEN** the user drags the sidebar from 250px down to 140px
- **THEN** the rows become avatar-only, the dashboard row icon-only, and the
  new-agent control icon-only

#### Scenario: Crossing back

- **WHEN** the user drags a compact sidebar back to 200px
- **THEN** every row shows its name and detail lines again

#### Scenario: The compact title bar is still identifiable

- **WHEN** the sidebar is compact and the pointer rests on its title bar
- **THEN** a tooltip names the workspace

#### Scenario: A compact row is still identifiable

- **WHEN** the pointer rests on a compact agent row whose agent is working on
  "Running tests"
- **THEN** a tooltip names that agent, says it is working, and shows
  "Running tests"

#### Scenario: A compact row still reports state

- **WHEN** an agent is working and the sidebar is compact
- **THEN** its state dot is visible on its avatar

#### Scenario: A stopped agent still reads as stopped when compact

- **WHEN** a workspace holds a running and a stopped agent and the sidebar is
  compact
- **THEN** the stopped agent's row is still visually distinguishable from the
  running one's

#### Scenario: Compact rows keep their menus

- **WHEN** the user right-clicks a compact agent row
- **THEN** the agent row context menu opens with the same items it has at full
  width

#### Scenario: Selection still shows

- **WHEN** an agent is selected and the sidebar is compact
- **THEN** its row is highlighted as the selected row

## MODIFIED Requirements

### Requirement: Selected agent shows a live terminal surface

The workspace window SHALL render the terminal contents of each agent shown
in a pane of its content area (see `workspace-layouts`), in place of the
current static placeholder; in a single layout that is the selected agent.
A non-shell agent's surface SHALL reflect the same PTY output already driving
its status/title tracking (`activity-detection`, `agent-hooks`).

#### Scenario: Selecting an agent shows its terminal

- **WHEN** the user selects an agent in the workspace sidebar
- **THEN** that agent's terminal surface becomes visible in the content pane
- **AND** the surface of any agent that is in no pane is hidden or detached

#### Scenario: Two terminals in a split

- **WHEN** a split layout shows two Terminal-mode agents
- **THEN** both agents' surfaces are visible and live, each in its own pane

#### Scenario: No agent selected

- **WHEN** no agent is selected in the workspace
- **THEN** the content pane shows no terminal surface (falls back to the
  existing "choose an agent" state)

### Requirement: Surface resizes with its container

The rendered surface SHALL track the size of the pane it occupies, including
window resizes, reflowing its row/column grid accordingly and resizing the
underlying PTY to match. While a divider between panes is being dragged, the
surface SHALL keep its grid, and SHALL resize once when the drag ends
(`workspace-layouts`, "Terminals resize when a drag ends").

#### Scenario: Window resize reflows the terminal grid

- **WHEN** the workspace window is resized
- **THEN** the visible terminal surface's row/column grid reflows to match
  its content pane's new size
- **AND** the underlying PTY is resized to match (so the running program
  sees the new dimensions)

#### Scenario: A divider drag resizes once

- **WHEN** the user drags the divider beside a terminal pane and releases it
- **THEN** the terminal's grid is unchanged during the drag, and its PTY is
  resized once, after the release

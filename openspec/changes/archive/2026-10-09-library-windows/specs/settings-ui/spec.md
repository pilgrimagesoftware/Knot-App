# Spec Delta

## MODIFIED Requirements

### Requirement: Window scope

The settings window SHALL show a tab strip with seven tabs — General,
Coding, Autopilot, Voice, MCP, Appearance, Keyboard — in that order, with
General selected by default when the window opens. Every tab SHALL render
its real pane; none render a placeholder. No "check for updates" control
SHALL be shown anywhere in the window.

Personas, Prompts and Bench are not tabs. Each has a window of its own (see
`library-windows`).

The Keyboard tab's contents are specified by `keybindings`. The Swift reference
has no Keyboard tab.

#### Scenario: General is the default tab

- **WHEN** the settings window opens
- **THEN** the General tab is selected and its four sections (Appearance,
  Startup, Notifications, Agent Panel) are visible

#### Scenario: Keyboard is the last tab

- **WHEN** the settings window opens
- **THEN** Keyboard is the seventh tab, after Appearance

#### Scenario: The libraries are not tabs

- **WHEN** the settings window opens
- **THEN** its tab strip has no Personas, Prompts or Bench tab

## REMOVED Requirements

### Requirement: Personas tab

**Reason**: The Personas list moved to a window of its own.

**Migration**: See `library-windows` - Personas window.

### Requirement: Prompts tab

**Reason**: The Prompts list moved to a window of its own.

**Migration**: See `library-windows` - Prompts window.

### Requirement: Bench tab

**Reason**: The Bench list moved to a window of its own.

**Migration**: See `library-windows` - Bench window.

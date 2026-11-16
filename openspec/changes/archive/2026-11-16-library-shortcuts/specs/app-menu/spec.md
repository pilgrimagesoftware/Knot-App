# Spec Delta

## MODIFIED Requirements

### Requirement: The Window menu opens the library windows

The Window menu SHALL carry Personas, Prompts and Bench items that open the
library windows (see `library-windows`). Each SHALL show its configurable
shortcut from `keybindings` as its key equivalent, following a customization.

Like Command Center and Workspaces, they SHALL be enabled at all times,
including when no window is open at all. Choosing one SHALL open its window,
or bring it to the front if it is already open, per `window-lifecycle`.

#### Scenario: Opening the Bench from the menu

- **WHEN** the user chooses Window > Bench
- **THEN** the Bench window opens, or comes to the front if it was already
  open

#### Scenario: Reachable with nothing focused

- **WHEN** every Knot window is closed and the user opens the Window menu
- **THEN** Personas, Prompts and Bench are all enabled

#### Scenario: The item shows its shortcut

- **WHEN** the user has not customized Open Bench and opens the menu
- **THEN** the Bench item shows ⌥⌘B

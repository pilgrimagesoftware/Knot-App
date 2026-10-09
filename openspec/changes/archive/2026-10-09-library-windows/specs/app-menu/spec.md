# Spec Delta

## ADDED Requirements

### Requirement: The Window menu opens the library windows

The Window menu SHALL carry Personas, Prompts and Bench items that open the
library windows (see `library-windows`). They SHALL have no key equivalents.

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

## MODIFIED Requirements

### Requirement: The Window menu's own items precede the window list

The Window menu SHALL list Knot's own items - Command Center, Workspaces,
Personas, Prompts, Bench, Minimize and Zoom - before the list of open windows
macOS appends to that menu, with a separator between Knot's items and that
list.

Command Center and Workspaces SHALL come first, as a group separated from
the rest. Personas, Prompts and Bench SHALL follow as a second group,
separated from what comes after. Minimize and Zoom come last. The first group
navigates between workspaces, the second opens the libraries, and the last
manipulates the focused window. None of them belong next to each other.

macOS appends and maintains the trailing list of open windows itself, so Knot's
items SHALL be declared in that order rather than positioned after the fact.
Without the separator, a workspace called "Zoom" is indistinguishable from the
Zoom command, and the menu's fixed items shift down every time a window opens.

#### Scenario: Order in the menu

- **WHEN** the user opens the Window menu
- **THEN** Command Center and Workspaces appear first, then a separator, then
  Personas, Prompts and Bench, then a separator, then the open workspace
  windows on their shortcuts and a separator when any is open, then Minimize
  and Zoom, then a separator, then the open windows

#### Scenario: The fixed items do not move

- **WHEN** the user opens three more workspace windows and opens the Window
  menu again
- **THEN** Command Center, Workspaces, Personas, Prompts, Bench, Minimize and
  Zoom are in the same positions, and the three new windows are listed below
  the last separator

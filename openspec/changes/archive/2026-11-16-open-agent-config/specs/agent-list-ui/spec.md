# Spec Delta

## ADDED Requirements

### Requirement: Config Files from the context menu

The agent row's context menu SHALL offer Config Files, a submenu immediately
after Markdown Files in the group that holds Open In, listing the agent
type's user-level config files and then its project files under the agent's
folder. Selecting one SHALL open it in the default text editor; one that
does not exist SHALL open nothing, without an error dialog. Config Files
SHALL be absent for an agent type with no known config files.

#### Scenario: Open a Claude agent's settings

- **WHEN** the user opens Config Files on a Claude agent and selects
  `~/.claude/settings.json`
- **THEN** that file opens in the default text editor

#### Scenario: A shell agent has no config files

- **WHEN** the user right-clicks a shell agent's row
- **THEN** Config Files is absent from the menu

#### Scenario: A project file that does not exist

- **WHEN** the user selects a project file the agent's folder does not have
- **THEN** nothing opens and no error dialog appears

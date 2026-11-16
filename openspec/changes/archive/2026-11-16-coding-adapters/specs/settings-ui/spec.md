# Spec Delta

## MODIFIED Requirements

### Requirement: Coding tab

The Coding tab SHALL show a "Source Folder" section (current
`source_base_folder`, a folder picker, and a clear action), then a row of
tabs, one per agent type: Claude, Codex, OpenCode, Gemini, Copilot and Shell.
Under the tabs, one group named for the selected agent SHALL hold everything
about it: an options field bound to `agent_options` for that type and, for an
agent with an installable ACP adapter, the adapter's command, its version
status, a check action and, when an update is available, an Update action.
The version status SHALL show the latest check's result, whether that check
ran in the background or on request. An agent's tab SHALL carry a dot while
an update is available for its adapter. The tab SHALL NOT show an "Open With"
section or editable custom-command fields - those depend on features not yet
in the Rust port.

The clear action SHALL ask for confirmation before clearing. Unlike every
other control on this tab, it discards a path the user chose through a file
dialog and cannot retype from memory.

#### Scenario: Choosing a source folder persists it

- **WHEN** the user picks a directory via "Choose…" in the Source Folder
  section
- **THEN** `source_base_folder` is saved as that directory's path
  immediately

#### Scenario: Clearing the source folder

- **WHEN** the user clicks the clear action next to a configured source
  folder and confirms the prompt
- **THEN** `source_base_folder` is saved as an empty string

#### Scenario: Cancelling the clear

- **WHEN** the user clicks the clear action and dismisses the prompt without
  confirming
- **THEN** `source_base_folder` is unchanged

#### Scenario: Editing options for an agent type persists it

- **WHEN** the user selects the Codex tab and types `--flag value` into its
  Options field
- **THEN** `agent_options["codex"]` is saved as `"--flag value"`
  immediately

#### Scenario: Switching agent type shows that type's own options

- **WHEN** `agent_options["claude"]` is `"--foo"` and the user switches from
  the Claude tab to the Codex tab (with no stored value yet)
- **THEN** the Options field shows empty, not `"--foo"`, and the group shows
  Codex's adapter, not Claude's

#### Scenario: A background check's result is shown

- **WHEN** the background check has found an update for Claude's adapter and
  the user opens the Coding tab
- **THEN** the Claude tab carries a dot and its version status shows the
  update, without the user checking again

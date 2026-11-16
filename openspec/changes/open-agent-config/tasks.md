# Tasks

## 1. Table

- [x] 1.1 `agent_config`: per-type user and project files, `$CODEX_HOME`,
      OpenCode's two extensions; pure
- [x] 1.2 `open_config_file`: first existing candidate, `open -t`, off the
      main thread; nothing for a missing file

## 2. Menus

- [x] 2.1 `AgentMenuEntry::ConfigFiles` after Markdown Files, shown only for a
      type with config files
- [x] 2.2 Context menu submenu
- [x] 2.3 Agents menu submenu and its `AgentMenuOpenConfigFile` action
- [x] 2.4 l10n keys for the entry and the project label

## 3. Tests

- [x] 3.1 Every roster coding agent has files; shell and custom have none
- [x] 3.2 Claude's order, Codex following `$CODEX_HOME`, OpenCode's two
      candidates, the project label substituting its path
- [x] 3.3 The entry follows the agent type; the full menu order includes it;
      the Agents menu lists the snapshot's files

## 4. Docs

- [x] 4.1 `docs/site/latest`: Config Files

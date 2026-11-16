# Proposal

## Why

#27 asks for a way to open an agent's configuration (`.claude.json`,
`opencode.jsonc` and the like) in an editor. Today that means knowing where
each agent type keeps its files and opening them by hand.

## What Changes

- The agent row's context menu, and with it the menu bar's Agents menu, gains
  **Config Files**, a submenu after Markdown Files. It lists the agent
  type's user-level config files, then its project files under the agent's
  folder (labelled as the project's).
- Selecting one opens it in the default text editor (`open -t`). An entry
  whose file doesn't exist does nothing, as Open In does for a missing
  application.
- Config Files is absent for a shell, a custom type, or a type Knot doesn't
  know.

Files per type:

| Type | User | Project |
| --- | --- | --- |
| Claude | `~/.claude.json`, `~/.claude/settings.json` | `.claude/settings.json`, `.claude/settings.local.json`, `.mcp.json` |
| Codex | `$CODEX_HOME/config.toml` (else `~/.codex`) | `.codex/config.toml` |
| OpenCode | `~/.config/opencode/opencode.jsonc` or `.json` | `opencode.jsonc` or `.json` |
| Gemini | `~/.gemini/settings.json` | `.gemini/settings.json` |
| Copilot | `~/.copilot/config.json`, `~/.copilot/mcp-config.json` | - |
| Antigravity | `~/.gemini/antigravity-cli/settings.json` | - |

## Impact

- `knot`: `agent_config` (the table, `config_files`, `open_config_file`);
  `AgentMenuEntry::ConfigFiles` and its fact; the context menu submenu; the
  Agents menu submenu and its `AgentMenuOpenConfigFile` action; two l10n keys.
- Specs: ADDED `agent-list-ui` "Config Files from the context menu". The
  Agents menu needs no delta: `app-menu` already requires it to carry every
  item the context menu can offer.

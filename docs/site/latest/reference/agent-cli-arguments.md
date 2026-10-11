---
title: Agent CLI arguments
version: latest
permalink: /latest/reference/agent-cli-arguments/
---

# Agent CLI arguments

Knot launches each coding-agent CLI with the flags defined for its type under
[Settings]({{ '/latest/settings/' | relative_url }}) - Coding - Agent Options,
and sends it Knot's own registration prompt and any startup prompt. This page
tracks the CLI arguments across the supported coding agents.

| Intent | Claude Code | Codex | OpenCode | Gemini CLI | GitHub Copilot |
|--------|-------------|-------|----------|------------|----------------|
| **System Prompt** | `--append-system-prompt "..."` | `-c 'developer_instructions=...'` | N/A | N/A | N/A |
| **User Prompt** | Last argument (no flag) | Last argument (no flag) | `--prompt "..."` | `--prompt-interactive "..."` | `--interactive "..."` |
| **MCP Config** | `--mcp-config '<json>'` | N/A | N/A | N/A | `--additional-mcp-config '<json>'` |
| **MCP Server Filter** | N/A | N/A | N/A | `--allowed-mcp-server-names` | N/A |
| **Allowed Tools** | `--allowed-tools 'mcp__knot__*'` | N/A | N/A | N/A | `--allow-tool 'knot(<tool>)'` (per tool) |
| **Hooks / Plugins** | `--plugin-dir "<path>"` | `-c 'notify=["bash","<script>"]'` | N/A | N/A | N/A |
| **Resume Conversation** | `--resume <session-id>` | `codex resume <thread-id>` | `-c` / `-s <session-id>` | `--resume <session-id>` | `--resume <session-id>` |
| **Fork Conversation** | `--fork-session` (with `--resume`) | `codex fork <thread-id>` | `--fork` (with `-c` or `-s`) | N/A | N/A |
| **Inline Registration** | Yes (system + user prompt) | Yes (system + user prompt) | Yes (user prompt) | Yes (user prompt) | Yes (user prompt) |
| **Activity Detection** | Hook-based (plugin) | Hook-based (notify script) | Terminal output | Terminal output | Terminal output |
| **Session ID Source** | `session_id` (hook payload) | `thread-id` (hook payload) | N/A | N/A | N/A |
| **History Storage** | JSONL in `~/.claude/projects/` | SQLite `~/.codex/state_5.sqlite` | N/A | `~/.gemini/tmp/<name>/logs.json` + `chats/` | `~/.copilot/session-state/<id>/workspace.yaml` |

Keep this table updated when adding new agents or discovering new CLI options.

## Source contracts

- `openspec/specs/agent-launch-command/spec.md`
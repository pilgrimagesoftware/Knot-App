# Tasks

## 1. Warn before a Codex agent's usage runs out

- [x] 1.1 Read Codex's latest rate-limit snapshot from its session files,
      tolerating any missing or changed data
- [x] 1.2 Refresh it off the main thread while a Codex agent is in the window,
      and repaint when it lands
- [x] 1.3 Show the near-limit banner with the reset time, "Get more usage" and
      dismiss; document it

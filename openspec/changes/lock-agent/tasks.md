# Tasks

## 1. Lock an agent out of messaging

- [x] 1.1 Persist `locked` on the agent; a duplicate or fork starts unlocked
- [x] 1.2 Refuse sends to and from a locked agent; skip it in broadcast and
      capability dispatch; report it in the registry
- [x] 1.3 Lock/Unlock in the context menu and the Agents menu
- [x] 1.4 Show Locked on the sidebar row; skip locked agents in the user's
      Broadcast to All Agents

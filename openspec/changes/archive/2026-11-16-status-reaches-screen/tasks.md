# Tasks

## 1. Repaint on a store change made off the UI thread

- [x] 1.1 Count status-text and state changes on `AgentStore`, only when the
      value changes; verify with store tests
- [x] 1.2 Repaint a window when the store's revision has passed the one it
      drew; verify two windows each see a change once, and that status text
      written straight to the store is noticed
- [x] 1.3 Give the Command Center a store poll of its own; verify a status
      written straight to the store repaints it once

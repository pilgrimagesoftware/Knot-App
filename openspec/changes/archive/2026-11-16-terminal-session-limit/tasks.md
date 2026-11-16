# Tasks

- [x] `SessionLimitTap`: anchored notice match over PTY bytes, escapes
      dropped, bounded line, one report per notice
- [x] Feed it for Claude agents in `spawn_pty`; `take_session_limit`
- [x] Window poll drains it into the store, limit before state
- [x] `AgentStore::set_tracked_state`; terminal tracker and hook writes use it
- [x] `UserPromptSubmit` clears the limit; Autopilot skips a limited agent
- [x] Tests, docs

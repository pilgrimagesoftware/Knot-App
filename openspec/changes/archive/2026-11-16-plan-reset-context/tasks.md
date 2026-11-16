# Tasks

- [x] `TaskSpec::fresh_context`, through `Task` and `Dispatchable`
- [x] `plan-tasks` parses `resetContext`; `TaskInfo` reports it
- [x] `dispatch-task` requests the restart after delivery; refused for the caller
- [x] `Agent::fresh_context_requested`, `request_fresh_context`, cleared by `restart`
- [x] Window restarts flagged agents between turns, before nudging; nudge held
- [x] Tests, docs

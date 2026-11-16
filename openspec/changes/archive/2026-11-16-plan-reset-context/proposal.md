# Proposal

## Why

A multi-step plan often hands one agent several steps in a row. Each step
then starts in the conversation the last one left: its files, its dead ends,
its half-finished reasoning, and a context window that only grows. Sometimes
that is what the plan wants; often a step should start clean (#331).

## What Changes

- A task in `plan-tasks` may carry `resetContext: true`.
- Dispatching such a task delivers its goal as before and asks for the
  assignee to be restarted on a new conversation. The window that owns the
  agent restarts it once it is not Working or awaiting input, so a previous
  step still under way is not cut short.
- The task waits in the agent's inbox through the restart, and the inbox
  nudge waits for it, so the agent reads the task in the new conversation.
- A caller cannot dispatch a `resetContext` task to itself: the restart
  would end the turn that dispatched it. Nor to an agent still holding
  another dispatched task: idle while it waits on a reply, it has already
  read that task, and the restart would lose it.
- A restart clears an agent's self-set working directory (#375), so plans
  should name the worktree in such a task's goal.
- `task-status` and `plan-tasks` report `resetContext` on the tasks that
  carry it.

## Impact

- `knot-tasks`: `TaskSpec::fresh_context`, carried to `Task` and
  `Dispatchable`.
- `knot-agents`: runtime `Agent::fresh_context_requested`;
  `AgentStore::request_fresh_context`; `AgentStore::restart` clears it.
- `knot-mcp-tools`: `plan-tasks` parses `resetContext`; `dispatch-task`
  requests the restart after delivery, and refuses it for the caller.
- `knot`: the poll restarts flagged agents before nudging, and holds their
  nudge until then.

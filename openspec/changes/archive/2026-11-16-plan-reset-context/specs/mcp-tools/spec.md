# Spec Delta

## ADDED Requirements

### Requirement: A task may start its assignee on a fresh conversation

A `plan-tasks` task MAY carry `resetContext` true. Dispatching it SHALL
deliver its goal as any dispatch does, and then restart the assignee without
its conversation as soon as the assignee is neither Working nor awaiting
input. The system SHALL NOT nudge the assignee to read its inbox until that
restart has happened. `plan-tasks` and `task-status` SHALL report
`resetContext` on each task that carries it.

#### Scenario: An idle assignee starts fresh

- **WHEN** a `resetContext` task is dispatched to an idle agent
- **THEN** the agent is restarted on a new conversation, and then nudged to
  read the task

#### Scenario: A working assignee finishes first

- **WHEN** a `resetContext` task is dispatched to an agent that is Working
- **THEN** the agent is not restarted until it stops working

#### Scenario: A task without it keeps the conversation

- **WHEN** a task without `resetContext` is dispatched
- **THEN** the assignee is not restarted

### Requirement: A reset does not cut work short

`dispatch-task` SHALL return `isError` true, leaving the task `ready`, when a
`resetContext` task's recipient is the caller, or holds another task in any
plan that is `dispatched` and not yet reported.

#### Scenario: The assignee is still on another task

- **WHEN** a `resetContext` task is dispatched to an agent idle while its
  previous dispatched task awaits a reply
- **THEN** the result has `isError` true, names that task, and the task stays
  `ready`

#### Scenario: Not to the caller

- **WHEN** the caller dispatches a `resetContext` task assigned to itself
- **THEN** the result has `isError` true and the task stays `ready`

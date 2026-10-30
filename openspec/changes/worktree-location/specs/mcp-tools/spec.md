# Spec Delta

## MODIFIED Requirements

### Requirement: create-agent

`create-agent` SHALL require the caller's `agentId`. It SHALL accept
`benchAgentId` (deploy a bench template; then name/agentType/repoPath are
optional), or otherwise require `name`, `agentType`, and `repoPath`. Optional
inputs: `icon`, `createWorktree` with `branchName` and `worktreePath`,
`companion`, `command` (shell only), `personaId`. `worktreePath` SHALL place the
new worktree as `create-worktree`'s `path` does; given without `createWorktree`
it SHALL be an error, and no agent SHALL be created. On success it SHALL create
the agent attributed to the caller and return the new agent id.

#### Scenario: Create from explicit fields

- **WHEN** `create-agent` is called with `name`, `agentType`, `repoPath`
- **THEN** an agent is created with `created_by` set to the caller and the new
  id is returned

#### Scenario: Create with a new worktree

- **WHEN** `create-agent` sets `createWorktree` true without `branchName`
- **THEN** the result is an error naming the missing `branchName`

#### Scenario: Create with a new worktree at a named location

- **WHEN** `create-agent` sets `createWorktree` with a `branchName` and a
  `worktreePath`
- **THEN** the worktree is created at `worktreePath` and the new agent's folder
  is that path

#### Scenario: A worktree location without a worktree

- **WHEN** `create-agent` is called with a `worktreePath` but without
  `createWorktree`
- **THEN** the result is an error naming `createWorktree`, and no agent is
  created

### Requirement: create-worktree

`create-worktree` SHALL require `repoPath` and a non-empty `branchName`, create
a worktree on a new branch, and return the new worktree path on success. It
SHALL accept an optional `path`: when given, the worktree SHALL be created
there, with a leading `~/` meaning the home folder; when omitted, at the
default sibling location. A `path` that is relative after that expansion SHALL
be an error, and no worktree SHALL be created.

#### Scenario: Empty branch name

- **WHEN** `create-worktree` is called with an empty `branchName`
- **THEN** the result is an error and no worktree is created

#### Scenario: Location named by the agent's instructions

- **WHEN** `create-worktree` is called with an absolute `path` whose parent
  folders do not exist yet
- **THEN** the worktree is created at `path` and nothing is created at the
  default location

#### Scenario: Relative location

- **WHEN** `create-worktree` is called with a relative `path`
- **THEN** the result is an error saying the path must be absolute, and no
  worktree is created

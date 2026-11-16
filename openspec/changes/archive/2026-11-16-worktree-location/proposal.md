# Proposal

## Why

`create-worktree` took only `repoPath` and `branchName`, so every worktree an
agent created went beside the repository as `<repo>-<branch>`. A project whose
agent instructions name a worktree location (Knot's own `CLAUDE.md` does) had
no way to be followed, and the agent was blamed for "choosing" a location it
was never asked for (#37). `create-agent` with `createWorktree` had the same gap.

## What Changes

- `create-worktree` accepts an optional absolute `path` for the new worktree;
  `create-agent` accepts the same as `worktreePath`. A leading `~/` is the
  home folder. Omitted, the sibling default is unchanged.
- A relative path is refused with an error, and nothing is created: the tool
  runs in Knot's process, so there is no agent working directory to resolve
  it against.
- Both tool descriptions tell the agent to pass the location its instructions
  give, since an agent only reads a parameter it knows to use.

## Impact

- `knot-mcp-tools`: `repos::create_worktree_path` takes the requested location;
  new `ToolError::RelativeWorktreePath`; catalog descriptions.
- Docs: `git-worktrees.md`, `coordination.md`.

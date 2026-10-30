# Tasks

## 1. Let an agent choose where its worktree goes

- [x] 1.1 Accept `path` on `create-worktree` and `worktreePath` on
      `create-agent`; verify the worktree is created there and not at the
      default
- [x] 1.2 Refuse a relative path without creating anything; expand `~/`
- [x] 1.3 Describe the parameter in the tool catalog and the user docs
- [x] 1.4 Refuse `worktreePath` on `create-agent` without `createWorktree`,
      creating no agent (#342)

# Tasks

- [x] 1.1 Answer a sessionless, non-`initialize` request without
      `Mcp-Session-Id`; keep the stale-id self-heal
- [x] 1.2 `items` on `describe-agents.capabilities` and `plan-tasks.tasks`
- [x] 2.1 Test agy's captured sequence: `server/discover`, `initialize`,
      `tools/list`
- [x] 2.2 Test that no array parameter in the catalog, at any depth, lacks
      `items`
- [x] 2.3 Mutation-check both
- [x] 3.1 Reproduce against agy 1.5.9 over ACP, through a logging proxy, with
      each fix applied alone and together

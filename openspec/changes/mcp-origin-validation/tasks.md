# Tasks

- [x] 1.1 Capture the `Origin` each supported agent sends (Claude Code,
      Codex, agy over ACP, opencode) through a logging proxy
- [x] 1.2 Refuse a non-loopback `Origin` with 403 and a JSON-RPC error on
      every route; serve a request with none
- [x] 2.1 Unit-test the rule: no Origin, loopback hosts on any port, other
      hosts, look-alike hosts, `null`, non-web schemes
- [x] 2.2 HTTP tests: no Origin and loopback served, foreign 403 on `/mcp`
      and both hook endpoints
- [x] 2.3 Mutation-check: without the layer the refusal tests fail

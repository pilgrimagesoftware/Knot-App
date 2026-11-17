# Tasks

- [x] 1.1 List the implemented revisions; `initialize` echoes a supported
      version and otherwise answers with the newest
- [x] 1.2 Refuse an unknown `MCP-Protocol-Version` with 400 on requests that
      carry a session; leave sessionless probes alone
- [x] 2.1 Unit-test negotiation and the header rule
- [x] 2.2 HTTP tests: the issue's reproduction, the 400, and agy's captured
      probe sequence still answered
- [x] 3.1 End to end through a logging proxy: agy 1.5.9 over ACP and Claude
      Code negotiate 2025-11-25, Codex 2025-06-18; each lists tools with its
      negotiated header

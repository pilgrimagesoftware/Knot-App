# Tasks

The design decisions are settled (design.md, "Decided questions"). Section
references (D1–D10) point to design.md.

## 1. Credentials (`knot-core`, `knot-mcp`)

- [x] 1.1 Credential registry:
  - 32 bytes from `getrandom`, base64url, with prefixes `knot_a_`, `knot_h_`
    and `knot_c_` (D1/D2).
  - Store SHA-256 digests mapped to a principal (`Agent(id)`,
    `AgentHooks(id)` or `External`) and the routes it is accepted on.
  - `issue_agent_pair(id)` and `rotate_agent_pair(id)` revoke and issue under
    one lock (D8). Also `revoke_agent(id)`, and `authenticate(&str) ->
    Option<Principal>` using `subtle` over every digest.
  - Constants in `consts.rs`.

  Verify: unit tests for issue, authenticate, revoke and rotate (the old pair
  is refused, the new pair accepted, and no window where both are valid,
  checked by a test that authenticates concurrently with rotation), plus an
  unknown credential and a wrong prefix. No `Debug`/`Display` shows a secret.
- [x] 1.2 Client token in the Keychain (`security-framework`, service
  `com.pilgrimagesoftware.knot.mcp-client-token`): load at startup, create
  if absent, regenerate. Verify: a test behind a trait seam with an
  in-memory store. The Keychain is exercised manually on a signed build.
- [x] 1.3 Add the three prefixes to the bug-report scrubber
  (`bug_report/scrub.rs`). Verify: a scrub test for each.
- [x] 1.4 Confirm nothing inside Knot calls `GET /api/v1/agent/status` over
  HTTP (D7), and record the finding in the PR.

## 2. Server middleware (`knot-mcp`)

- [x] 2.1 Host check (D4 step 1) before Origin. Verify: HTTP tests for
  `other.example:<port>`, `127.0.0.1:<other port>` and a missing Host
  (each → 403), and for `localhost:<port>` and `127.0.0.1:<port>` (both
  pass).
- [x] 2.2 Auth middleware (D4 step 3) on every route except `/health`: 401
  with `WWW-Authenticate: Bearer realm="knot"`, and 403 for a credential
  outside its route scope. Verify: an HTTP test for each route (`/`, `/mcp`
  POST and GET, the status GET, both hook POSTs) with no credential, an
  unknown one, each kind, and the correct kind. Also `/health` with none.
- [x] 2.3 Principal versus `?agent=` hint → 403 on mismatch (D4 step 4).
  Verify: an HTTP test.
- [x] 2.4 Log the principal and the outcome, never a credential. Attribute
  refusals to the `?agent=` hint, or to the `User-Agent` when there is no
  hint (D10). Verify: a log test that no line contains `knot_a_`, `knot_h_`
  or `knot_c_`, and a counter test of the attribution.

## 3. Session binding and identity (`knot-mcp`, `knot-mcp-tools`)

- [x] 3.1 `session.rs` `bind` keyed by principal (D5). Verify: unit tests
  for restart takeover with the new credential, an External attempt refused,
  and an `Agent(y)` attempt for x refused.
- [x] 3.2 `identity.rs` takes the caller from the principal. Update the
  identity tests to the `mcp-tools` deltas.
- [x] 3.3 Run the MCP HTTP integration suite (`crates/knot-mcp/tests/`) with
  credentials threaded through the fixtures.

## 4. Launch path (`knot-agent-launch`, `knot-acp`, `knot-terminal`, `plugin/`)

- [x] 4.1 Issue the pair at launch, rotate on Restart and Restart with New
  Conversation (D8), and revoke on close. The tool credential goes in
  `mcpServers[].headers` on `session/new` and `session/load`; the hook
  credential in the adapter environment.
  - Keep the header-feasibility probe as `scripts/acp-header-probe.py` (no
    credentials, a dummy value only).
  - Re-run it for Claude, Codex and agy. Run it for Copilot before Copilot
    gets a tool credential. Done 2026-11-26: Copilot CLI 1.0.116 sends the
    header on every request and puts no credential on a command line (probe
    run by the maintainer locally), so Copilot is wired like the others
    (`sends_mcp_headers`).
  - Check that agy keeps working when its header-less stream GET gets 401.
  - Update the design.md table with the versions used.

  Verify:
  - a client test that the header carries the tool credential and the
    environment carries only the hook credential;
  - a launch-level test that after a restart the old tool credential gets
    401.
- [x] 4.2 Notice for the Claude argv-delivery configurations
  (`agent-launch-command` "Adapter-side credential exposure is surfaced").
  Verify: a unit test for each trigger (the environment variable, a
  `settings` option, an `sdk-url` option) and for none.
- [x] 4.3 Migrate `plugin/claude/scripts/*.sh` and
  `plugin/codex/scripts/*.sh` from `SKWAD_*`/8766 to `KNOT_MCP_URL` and
  `KNOT_MCP_TOKEN`, with `Authorization: Bearer`. Verify: `shellcheck`, and an
  HTTP test that a hook with the hook credential is accepted and one for
  another agent's id gets 403.

## 5. Confirmation (`knot-mcp-tools`, `knot`)

- [x] 5.1 `create-agent` with a shell `shellCommand`, and worktree creation
  with a canonical destination outside the repository's parent, wait on a
  confirmation request to the UI (D9), with a timeout constant in
  `consts.rs`. A denial or timeout is a tool error with nothing created.
  Verify: tool tests for confirm, deny and timeout, and for a plain shell and
  a sibling worktree going through with no prompt.
- [x] 5.2 Confirmation dialog naming the requesting agent, the command and
  the folder. Verify: a gpui test that confirm and deny resolve the request.
  Assert l10n keys.

## 6. Settings, panel and docs (`knot`, `docs/site`)

- [x] 6.1 MCP tab:
  - masked token with copy, and regenerate with a confirmation;
  - install commands with the header per client, each CLI's syntax checked
    against its current version;
  - the refused-attempt count with agent attribution;
  - a note that the token is stored in each client's configuration.

  Verify: gpui tests for mask and copy, regenerate (the old token refused),
  and the command text per type. Assert l10n keys.
- [x] 6.2 Agent panel notice "Knot tools unavailable" when that agent's MCP
  connection was refused (D10). Verify: a panel test.
- [x] 6.3 Docs: the MCP setup page (token, client configuration storage,
  re-running the install command after upgrading) and `coordination.md`.
  Add a release-notes line through the changelog flow.

## 7. Gate

- [ ] 7.1 `make` passes (fmt-check, size-check, lint, test, build).
- [ ] 7.2 Security co-review of the implementation PR against D1–D10.

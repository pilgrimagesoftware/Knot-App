# Tasks

Open questions Q1–Q5 in design.md are decided before task 2 starts. Q1 adds
task 6, and Q3 sets the scope of task 4.2.

## 1. Credentials (`knot-core`, `knot-mcp`)

- [ ] 1.1 Credential type and registry:
  - 32 bytes from `getrandom`, base64url, `knot_a_`/`knot_c_` prefixes.
  - The registry stores SHA-256 digests mapped to principal (`Agent(id)` |
    `External`).
  - `issue_agent(id)`, `revoke_agent(id)` and `authenticate(&str) ->
    Option<Principal>` using `subtle` constant-time comparison over every
    digest.
  - Constants in `consts.rs`.

  Verify: unit tests for issue, authenticate, revoke, an unknown credential
  and a wrong prefix. No `Debug` or `Display` shows the secret.
- [ ] 1.2 Client token in the Keychain (`security-framework`, service
  `com.pilgrimagesoftware.knot.mcp-client-token`): load on startup, create if
  absent, regenerate. Verify: a test behind a trait seam with an in-memory
  store. The Keychain is exercised manually on a signed build.
- [ ] 1.3 Add the credential patterns to the bug-report scrubber
  (`bug_report/scrub.rs`). Verify: a scrub test with both prefixes.
- [ ] 1.4 Confirm nothing inside Knot calls `GET /api/v1/agent/status`
  over HTTP (D7). If something does, it presents the client token.

## 2. Server middleware (`knot-mcp`)

- [ ] 2.1 Host check (D4 step 1) as middleware before Origin. Verify: an
  HTTP test for `Host: attacker.example:<port>` → 403, a missing Host → 403,
  and `localhost:<port>` and `127.0.0.1:<port>` → pass.
- [ ] 2.2 Auth middleware (D4 step 3) on every route except `/health`: 401
  with `WWW-Authenticate: Bearer realm="knot"` and a JSON-RPC error with no
  id. Store the principal in request extensions. Verify: an HTTP test for
  each route with no, unknown and valid credentials, and `/health` with
  none.
- [ ] 2.3 Principal versus `?agent=` hint → 403 on mismatch (D4 step 4).
  Verify: an HTTP test.
- [ ] 2.4 Log the principal and the auth outcome, never the credential.
  Count 401s per `User-Agent` over a rolling hour for the settings tab.
  Verify: a log test asserts that no `knot_` prefix appears in any line.

## 3. Session binding and identity (`knot-mcp`, `knot-mcp-tools`)

- [ ] 3.1 `session.rs` `bind` keyed by principal: `Agent(x)` replaces only a
  session that `Agent(x)` bound, and `External` never evicts (D5). Verify:
  unit tests for restart takeover, the External attempt refused, and an
  Agent(y) attempt for x refused.
- [ ] 3.2 `identity.rs`: the caller comes from the principal. An External
  caller naming an agent with a live `Agent` session is refused. Update the
  existing identity tests to the `mcp-tools` deltas.
- [ ] 3.3 Run the full MCP HTTP integration suite (`crates/knot-mcp/tests/`)
  with credentials threaded through the fixtures.

## 4. Launch path (`knot-agent-launch`, `knot-acp`, `knot-terminal`)

- [ ] 4.1 Issue the agent credential when an ACP session starts, and revoke
  it when the session ends or the agent closes. Put it in
  `mcpServers[].headers` on `session/new` and `session/load`. Verify:
  - a client test asserts the header is present and the URL has no secret;
  - a live check, per adapter that declares `mcpCapabilities.http`, that
    `set-status` succeeds (record each adapter and version in the PR).
- [ ] 4.2 Set `KNOT_MCP_URL` and `KNOT_MCP_TOKEN` in the adapter subprocess
  environment, and move `plugin/*/scripts` from `SKWAD_*` to them, adding
  the `Authorization` header (scope per Q3). Verify: shellcheck the scripts,
  and an HTTP test for a hook with each credential kind and one for another
  agent's id → 403.

## 5. Settings and docs (`knot`, `docs/site`)

- [ ] 5.1 MCP tab:
  - masked token with copy, and regenerate with a confirmation;
  - install commands with the header per client (verify each CLI's syntax
    against its current version, Q5);
  - the count of refused attempts.

  Verify: a gpui test for mask and copy, regenerate (the old token is
  refused), and the command text for each type (assert l10n keys, not
  English).
- [ ] 5.2 Docs: the MCP setup page (token, re-running the install command
  after upgrading) and `coordination.md`. Add a release-notes line under the
  next version through the normal changelog flow.

## 6. Confirmation for MCP-created shell agents and worktrees (only if Q1 = a)

- [ ] 6.1 `create-agent` with `agentType: shell` and a non-empty
  `shellCommand`, and `create-worktree` with a destination outside the
  repo's parent directory, wait for a user confirmation that shows the
  command and folder. A denial returns a tool error. Verify: tool tests for
  confirm and deny, with a gpui test for the dialog.

## 7. Gate

- [ ] 7.1 `make` passes (fmt-check, size-check, lint, test, build).
- [ ] 7.2 Security co-review of the implementation PR against design.md
  D1–D7.

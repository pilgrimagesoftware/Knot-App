# Antigravity recognizer fixtures

Captured from a live `agy acp` 1.5.8 session (paths and ids shortened). The
dispatch and completion are the `tool_call` and `tool_call_update` for one
`invoke_subagent` call; `not_a_delegation` is the shape of an ordinary tool
call. A failed delegation was not observed, so it has no fixture and the
recognizer's failure arm is the generic ACP `failed` status.

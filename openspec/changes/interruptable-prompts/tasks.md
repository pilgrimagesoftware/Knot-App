# Tasks

## 1. Protocol

- [x] 1.1 Keep `InitializeResult`'s top-level `_meta` and derive
      `AgentCapabilities::supports_steering` from `steering.supported`
- [x] 1.2 `AcpClient::session_steer(session_id, text) -> SteerOutcome`
      (`Injected`, `StartedNewTurn`, `PromptRequired`), sending
      `idleBehavior: "promptRequired"`, refusing locally when unsupported
- [x] 1.3 `AcpSession` pass-through

## 2. Queue

- [x] 2.1 A `steering` state on `QueuedPanelPrompt`: not drained at turn end
      and not steered twice
- [x] 2.2 Steer now on a queued entry, shown only for an agent that supports
      steering and only while a turn runs
- [x] 2.3 Outcomes: injected or startedNewTurn records the prompt in the
      running turn and removes the entry; promptRequired returns it to the
      head; an error marks it failed

- [x] 2.4 Offer Steer now only while no permission request is pending, per
      "Permission gate is independent of the queue" (#419 review)
- [x] 2.5 Map `codex-acp`'s `{"outcome": "failed"}` to a failed entry

## 3. Tests

- [x] 3.1 `initialize` with and without `_meta.steering.supported`, using the
      adapter's own response shape as the fixture
- [x] 3.2 A fake adapter answering `injected`, `promptRequired` and an error;
      the queue state after each
- [x] 3.3 Steer now is absent for an agent that does not advertise steering
- [x] 3.4 A pending permission offers no Steer now and sends nothing; a
      `failed` outcome leaves the entry queued and marked; mutation-checked

## 4. Docs

- [x] 4.1 `docs/site/latest`: Steer now, and which agents offer it

## 5. Follow-ups (not in this change)

- [ ] 5.1 Show background work from ACP v2 `state_update.background`, with
      per-item cancel, once adapters ship it
- [ ] 5.2 A control that moves a running tool call to the background, once ACP
      has a request for it

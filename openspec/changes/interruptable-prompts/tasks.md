# Tasks

## 1. Protocol

- [ ] 1.1 Keep `InitializeResult`'s top-level `_meta` and derive
      `AgentCapabilities::supports_steering` from `steering.supported`
- [ ] 1.2 `AcpClient::session_steer(session_id, text) -> SteerOutcome`
      (`Injected`, `StartedNewTurn`, `PromptRequired`), sending
      `idleBehavior: "promptRequired"`, refusing locally when unsupported
- [ ] 1.3 `AcpSession` pass-through

## 2. Queue

- [ ] 2.1 A `steering` state on `QueuedPanelPrompt`: not drained at turn end
      and not steered twice
- [ ] 2.2 Steer now on a queued entry, shown only for an agent that supports
      steering and only while a turn runs
- [ ] 2.3 Outcomes: injected or startedNewTurn records the prompt in the
      running turn and removes the entry; promptRequired returns it to the
      head; an error marks it failed

## 3. Tests

- [ ] 3.1 `initialize` with and without `_meta.steering.supported`, using the
      adapter's own response shape as the fixture
- [ ] 3.2 A fake adapter answering `injected`, `promptRequired` and an error;
      the queue state after each
- [ ] 3.3 Steer now is absent for an agent that does not advertise steering

## 4. Docs

- [ ] 4.1 `docs/site/latest`: Steer now, and which agents offer it

## 5. Follow-ups (not in this change)

- [ ] 5.1 Show background work from ACP v2 `state_update.background`, with
      per-item cancel, once adapters ship it
- [ ] 5.2 A control that moves a running tool call to the background, once ACP
      has a request for it

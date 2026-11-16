# Tasks

## 1. Data and evaluation

- [ ] 1.1 `Schedule`, `Preset`, `IdlePolicy`, `IdleAction` in `knot-core` with `Display`/`FromStr`; `SavedAgent.schedule` and `.idle_policy` with `#[serde(default)]`; `Settings.idle_default`
- [ ] 1.2 ADR: cron evaluation over `jiff` (recommended) vs `croner` + `chrono`
- [ ] 1.3 `next_run` for presets and five-field cron in local time; DST tests; cron property test against brute force
- [ ] 1.4 Persistence tests: older records and preferences load as no schedule, `Inherit`, off

## 2. Scheduled start

- [ ] 2.1 App-level ticker (30 s) with the due/skip/grace/missed decision as a pure, tested function
- [ ] 2.2 On a due run: `open_agent`, then `restart_agents(.., false)`; skip when mid-turn; record last fired run per agent

## 3. Idle shutdown

- [ ] 3.1 One per-agent runtime struct (last user activity, last fired run, idle sample), pruned in `teardown_session`
- [ ] 3.2 Stamp user activity from composer keys, sends, terminal keys and scroll in conversation and terminal
- [ ] 3.3 Once-a-minute process sample for agents with an idle policy, joined to `repaint_poll_tick`'s chain
- [ ] 3.4 Idle start as a pure function of the signals; evaluation every 15 s per window; deactivate or reset
- [ ] 3.5 Reset suppresses the startup prompt for that restart only; tests for both paths

## 4. UI

- [ ] 4.1 Agent editor: schedule (none/preset/cron with next-run preview and validation) and idle policy; not for shell agents
- [ ] 4.2 Settings: idle default control, applied live
- [ ] 4.3 Sidebar clock and timer detail lines, compact tooltip, title-bar timer icon; catalog keys with substitution tests

## 5. Docs

- [ ] 5.1 `docs/site/latest/`: scheduling an agent, cron examples, idle shutdown, what counts as idle, that Knot must be running

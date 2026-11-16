# Tasks

## 0. Session signal

- [x] 0.1 Replace `DashboardAgent.is_running` (`activated`) with `has_session`: the workspace dashboard reads its own session maps; the Command Center reads them through `WindowRegistry::workspace_view` and a `pub(crate) WorkspaceWindow::has_session`. Feed it to the card's working indicator.
- [x] 0.2 Rebuild the Command Center's snapshot when a workspace window opens or closes, not only on store revisions
- [x] 0.3 Tests: window open with a slot, open without one, closed with `activated` set

## 1. Status summary

- [x] 1.1 `status_counts` over `DashboardAgent`, through `working_indicator::state`; unit tests for order, dropped zeros, shell agents and not-running agents
- [x] 1.2 Draw it beside each section's name, and in the header of the Command Center only
- [x] 1.3 Catalog keys per state through `l10n::pluralize`; key-resolution tests

## 2. Quick prompt

- [ ] 2.1 One `InputState` per panel-agent card in the drawing view, pruned with its roster
- [ ] 2.2 The field on the card: state table (not drawn, disabled, idle, queues) as a pure function of the snapshot, with tests
- [ ] 2.3 `pub(crate) WorkspaceWindow::quick_prompt` calling `deliver_panel_prompt(.., PromptOrigin::User)`; Enter and Cmd+Enter through it; keep the text when no session or window is found
- [ ] 2.4 Command Center routing through `WindowRegistry::workspace_view`
- [ ] 2.4a The "will queue" placeholder for working, awaiting input and session-limit holds
- [ ] 2.5 Grow `CARD_HEIGHT` for every card; stop click propagation from the field
- [ ] 2.6 Window tests: idle agent sends, busy agent queues with origin `User`, Command Center reaches the owning window

## 3. Docs

- [ ] 3.1 Describe the summary and the quick prompt in `docs/site/latest/`

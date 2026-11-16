# Tasks

## 1. Status summary

- [ ] 1.1 `status_counts` over `DashboardAgent`, through `working_indicator::state`; unit tests for order, dropped zeros, shell agents and not-running agents
- [ ] 1.2 Draw it in the dashboard header and beside each workspace section's name, in the workspace dashboard and the Command Center
- [ ] 1.3 Catalog keys per state through `l10n::pluralize`; key-resolution tests

## 2. Quick prompt

- [ ] 2.1 One `InputState` per panel-agent card in the drawing view, pruned with its roster
- [ ] 2.2 The field on the card: state table (not drawn, disabled, idle, queues) as a pure function of the snapshot, with tests
- [ ] 2.3 Enter and Cmd+Enter through `deliver_panel_prompt(.., PromptOrigin::User)`; keep the text when no session is found
- [ ] 2.4 Command Center routing through `WindowRegistry::workspace_view`
- [ ] 2.5 Grow `CARD_HEIGHT` for every card; stop click propagation from the field
- [ ] 2.6 Window tests: idle agent sends, busy agent queues with origin `User`, Command Center reaches the owning window

## 3. Docs

- [ ] 3.1 Describe the summary and the quick prompt in `docs/site/latest/`

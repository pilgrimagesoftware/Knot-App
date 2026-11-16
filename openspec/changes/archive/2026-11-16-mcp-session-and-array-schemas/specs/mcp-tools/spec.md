# Spec Delta

## ADDED Requirements

### Requirement: Array parameters declare their items

Every tool's input schema SHALL give each array, at any depth, an `items`
schema describing its elements, because function-calling backends such as
Gemini's reject a request carrying an array parameter without one.

#### Scenario: plan-tasks

- **WHEN** a client lists the tools
- **THEN** `plan-tasks`' `tasks` declares object items with `id`, `goal`,
  `assignee`, `capabilities`, `dependsOn` and `resetContext`, and
  `describe-agents`' `capabilities` declares string items

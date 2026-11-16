# Spec Delta

## MODIFIED Requirements

### Requirement: Tool catalog

`tools/list` SHALL return exactly these tools: `register-agent`,
`list-agents`, `describe-agents`, `send-message`, `check-messages`,
`broadcast-message`, `list-repos`, `list-worktrees`, `create-agent`,
`close-agent`, `create-worktree`, `set-status`, `display-markdown`,
`view-mermaid`, `show-checklist`, `get-checklist`, `plan-tasks`,
`dispatch-task`, `complete-task`, `task-status`. Every tool SHALL declare a
JSON object input schema with typed properties and a required list.

#### Scenario: Catalog is complete and stable

- **WHEN** `tools/list` is called
- **THEN** all twenty tools are present, each with a name, description, and
  object input schema

## ADDED Requirements

### Requirement: show-checklist

`show-checklist` SHALL require `agentId` and `items` and accept an optional
`title`. `items` SHALL be a non-empty array of objects, each with a non-empty
`id` unique within the list, a non-empty `text`, and an optional `kind` of
`check` (the default) or `question`. A malformed list SHALL be a tool error
naming the offending item and SHALL leave the agent's checklist unchanged. A
valid call SHALL set the agent's checklist, unsent, in its artifact panel.

#### Scenario: Show a checklist

- **WHEN** `show-checklist` is called with a check and a question
- **THEN** the agent's artifact panel shows both, unticked and unanswered

#### Scenario: A malformed list is refused

- **WHEN** `show-checklist` is called with two items sharing an id
- **THEN** the call is a tool error naming the duplicate and no checklist is set

### Requirement: show-checklist keeps the user's answers on update

Calling `show-checklist` for an agent that already has a checklist SHALL
replace its items while keeping the user's tick or answer for every item whose
`id` and `kind` are unchanged. It SHALL NOT close or replace the agent's
markdown or diagram artifacts.

#### Scenario: An update keeps the answers

- **WHEN** the user has ticked an item and `show-checklist` is called again
  with that item's id and kind and reworded text
- **THEN** the item shows the new text and is still ticked

#### Scenario: A changed kind starts over

- **WHEN** an answered question is shown again with the same id as a check
- **THEN** the check is unticked

### Requirement: get-checklist

`get-checklist` SHALL require `agentId` and return the agent's checklist as
JSON: its `title`, whether it has been `submitted` since it last changed, and
every item with its `id`, `text` and `kind`, plus `checked` for a check and
`answer` for a question. An agent with no checklist SHALL get a tool error that
names `show-checklist`.

#### Scenario: Read the user's answers

- **WHEN** the user has ticked a check and answered a question, and the agent
  calls `get-checklist`
- **THEN** the result shows the check as ticked and the question's answer,
  without the user having sent anything

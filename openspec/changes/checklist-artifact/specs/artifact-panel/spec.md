# Spec Delta

## ADDED Requirements

### Requirement: The checklist section collects the user's answers

An agent's checklist (set by `show-checklist`) SHALL be an artifact of the
panel: alone it SHALL open and fill the panel; beside a markdown or diagram
section it SHALL have its own area below them, sharing the height evenly, and
SHALL NOT take part in their split or collapse. A check SHALL be a checkbox and
a question a text field. Each tick and answer SHALL be recorded on the agent's
checklist as it is made, and each field SHALL show the answer the checklist
holds.

#### Scenario: A checklist alone opens the panel

- **WHEN** an agent with no other artifact shows a checklist
- **THEN** its artifact panel opens with the checklist filling it

#### Scenario: A field follows a replaced checklist

- **WHEN** the user has typed an answer, closed only the checklist, and the
  agent shows the same items again
- **THEN** the answer field is empty, as the new checklist's answer is

### Requirement: Send delivers the checklist to the agent

The checklist's header SHALL carry a Send control. Send SHALL deliver the whole
checklist as one message - each item with its id and its tick or answer - the
way the markdown section's Approve delivers: a prompt for a Panel-mode agent, a
submitted command for a Terminal-mode one. The checklist SHALL be marked sent
only once delivered, and Send SHALL then be unavailable until a tick or answer
changes.

#### Scenario: Answers are sent as one message

- **WHEN** the user ticks an item, answers a question, and presses Send
- **THEN** the agent receives one message listing every item with its state and
  the checklist is marked sent

#### Scenario: Nothing to deliver to

- **WHEN** the user presses Send for an agent with no session
- **THEN** the checklist is not marked sent

### Requirement: A closed checklist keeps its answers

Closing the checklist SHALL remember it with its ticks and answers, and
reopening the panel SHALL restore it as it was. It SHALL carry its own close
control only when another section is open, as the other sections do.

#### Scenario: Reopening keeps the answers

- **WHEN** the user closes a checklist they have partly answered and then
  reopens the panel
- **THEN** the checklist returns with the same ticks and answers

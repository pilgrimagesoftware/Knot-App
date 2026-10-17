# Spec Delta

## ADDED Requirements

### Requirement: The checklist section collects the user's answers

An agent's checklist (set by `show-checklist`) SHALL be an artifact of the
panel: a checklist alone SHALL open the panel, and SHALL fill it. Beside a
markdown or diagram section it SHALL have its own area below them, sharing the
panel's height evenly, and SHALL NOT take part in their split or collapse. Its
header SHALL carry its title and a Send control, and its own close control only
when another section is open, as the other sections' do.

A check SHALL be a checkbox the user ticks; a question SHALL be a text field the
user types an answer into. Each tick and answer SHALL be recorded on the agent's
checklist as it is made, so `get-checklist` reads it at once.

Send SHALL deliver the whole checklist to the agent as one message - each item
with its tick or answer, and its id - the way the markdown section's Approve
delivers: as a prompt for a Panel-mode agent and as a submitted command for a
Terminal-mode agent. Sending SHALL mark the checklist sent, and Send SHALL be
unavailable until a tick or answer changes.

Closing the checklist SHALL remember it with its ticks and answers, and
reopening the panel SHALL restore it as it was.

#### Scenario: A checklist alone opens the panel

- **WHEN** an agent with no other artifact shows a checklist
- **THEN** its artifact panel opens with the checklist filling it

#### Scenario: Answers are sent as one message

- **WHEN** the user ticks an item, answers a question, and presses Send
- **THEN** the agent receives one message listing every item with its state and
  the checklist is marked sent

#### Scenario: Reopening keeps the answers

- **WHEN** the user closes a checklist they have partly answered and then
  reopens the panel
- **THEN** the checklist returns with the same ticks and answers

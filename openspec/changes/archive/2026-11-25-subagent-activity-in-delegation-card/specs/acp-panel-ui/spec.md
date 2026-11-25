## ADDED Requirements

### Requirement: A subagent's activity stays inside its delegation card

When an agent reports a tool call or text as made by a subagent it launched
(claude-agent-acp: `_meta.claudeCode.parentToolUseId`), the panel SHALL show it
inside the card of the delegation call with that id, never in the
conversation, so the main agent's reply stays one piece.

#### Scenario: A subagent runs while the main agent writes

- **WHEN** the main agent streams a reply containing a table while a subagent
  it launched reports tool calls and text between the reply's chunks
- **THEN** the reply is one message with the table intact, and the subagent's
  tool calls and text appear in order inside the delegation card

#### Scenario: A background subagent after the turn

- **WHEN** a subagent launched in the background reports after the turn that
  launched it ended
- **THEN** its report still appears inside its delegation card

### Requirement: A delegation card counts and lists its subagent's activity

A delegation card SHALL show a count line of its subagent's tool calls,
collapsed or open, in the words of a compact tool-call summary. While open it
SHALL list the subagent's tool calls and text in arrival order, each tool call
as a card with its own open/closed state.

#### Scenario: The delegation card counts its subagent's calls

- **WHEN** a subagent has run two commands
- **THEN** its delegation card shows a count line naming two tool calls and
  two commands run, collapsed or open

#### Scenario: A nested tool call opens on its own

- **WHEN** the user opens a nested tool call inside an open delegation card
- **THEN** that call's output shows, and the delegation card and its other
  nested calls keep their own state

### Requirement: A subagent report ahead of its delegation card is held, never dropped

A subagent's report that arrives before its delegation card SHALL move into
the card if the card arrives later in the turn. One still waiting SHALL appear
in the conversation when the turn ends, normally or in an error, or when the
session ends.

#### Scenario: A report ahead of its delegation card

- **WHEN** a subagent's tool call arrives before the delegation card that
  launched it, in the same turn
- **THEN** the tool call moves into that card when the card arrives

#### Scenario: A delegation card that never arrives

- **WHEN** a turn ends while a subagent's report is still waiting for its
  delegation card
- **THEN** the report is shown in the conversation rather than dropped

#### Scenario: A turn that ends in an error

- **WHEN** a turn the agent answers with an error ends while a subagent's
  report is still waiting for its delegation card
- **THEN** the report is shown in that turn's conversation, before the next
  prompt

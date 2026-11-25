# Spec Delta

## ADDED Requirements

### Requirement: Deactivate with New Session
The system SHALL offer Deactivate with New Session for a running agent that
is not a shell. After confirmation it SHALL deactivate the agent exactly as
"Deactivating an agent" does and clear its session ids, so the agent's next
start begins a new conversation. It SHALL NOT start the agent.

#### Scenario: The next start is a new conversation
- **WHEN** the user confirms Deactivate with New Session on an agent with
  ACP session id `a1`, and later selects the agent
- **THEN** the agent starts, and it starts a new session rather than
  resuming `a1`

#### Scenario: The prompt says the conversation is not kept
- **WHEN** the user chooses Deactivate with New Session
- **THEN** a confirmation appears saying the agent's current conversation
  will not be resumed

#### Scenario: Cancelling leaves the agent running
- **WHEN** the user chooses Deactivate with New Session and dismisses the
  prompt
- **THEN** the agent is still running, with its session unchanged

#### Scenario: Plain Deactivate still resumes
- **WHEN** the user chooses Deactivate on an agent with ACP session id
  `a1`, and `restore-conversation-on-launch` is enabled, and later selects
  the agent
- **THEN** the agent resumes `a1`

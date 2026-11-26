# Spec Delta

## ADDED Requirements

### Requirement: Deactivate with New Session
The system SHALL offer Deactivate with New Session for a running agent that
is not a shell, while `restore-conversation-on-launch` is enabled. After
confirmation it SHALL deactivate the agent exactly as "Deactivating an
agent" does and clear its session ids, so the agent's next start begins a
new conversation. It SHALL NOT start the agent.

#### Scenario: The next start is a new conversation
- **WHEN** the user confirms Deactivate with New Session on a running agent
  with ACP session id `a1`, and later selects the agent
- **THEN** the agent starts, and it starts a new session rather than
  resuming `a1`

#### Scenario: The prompt says the conversation is not kept
- **WHEN** the user chooses Deactivate with New Session on a running agent
- **THEN** a confirmation appears saying the agent's current conversation
  will not be resumed

#### Scenario: Cancelling leaves the agent running
- **WHEN** the user chooses Deactivate with New Session on a running agent
  and dismisses the prompt
- **THEN** the agent is still running, with its session unchanged

#### Scenario: Plain Deactivate still resumes
- **WHEN** the user chooses Deactivate on a running agent with ACP session
  id `a1`, and `restore-conversation-on-launch` is enabled, and later
  selects the agent
- **THEN** the agent resumes `a1`

### Requirement: A session id from before a new session is not kept
A session id reported by a connection that started before the agent was
restarted, or deactivated with a new session, SHALL NOT be stored for the
agent. A session id reported after a plain Deactivate SHALL still be
stored, since that agent keeps its resume point.

#### Scenario: A late id after Deactivate with New Session
- **WHEN** an agent's connection is still starting, the user confirms
  Deactivate with New Session, and the connection then reports session id
  `a2`
- **THEN** the agent has no ACP session id, and its next start is a new
  session

#### Scenario: A late id after Restart with New Conversation
- **WHEN** an agent's connection is still starting, the user confirms
  Restart with New Conversation, and the old connection then reports
  session id `a2`
- **THEN** `a2` is not stored for the agent

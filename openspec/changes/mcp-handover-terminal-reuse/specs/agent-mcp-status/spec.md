## MODIFIED Requirements

### Requirement: A server needing attention delegates to the agent's own flow

A row for one of the agent's servers in the needs-authentication, failed,
pending-approval or disabled state SHALL offer an action that hands the user
that agent type's own MCP flow. The system SHALL open a terminal in the
agent's working directory and enter that agent type's MCP command into it,
leaving the user in that flow.

Where the agent type offers a command that addresses one named server
directly, the action SHALL use it, naming the server on the row. Where it
offers only an interactive flow, the action SHALL enter whatever reaches that
flow for the type. Either way the user completes the flow themselves: the
system SHALL NOT perform authentication, SHALL NOT store credentials, and
SHALL NOT disturb the agent's ACP session.

When that terminal exits, the system SHALL probe again so the section
reflects whatever the user did.

A row in the connected or unknown state SHALL NOT offer the action, and
neither SHALL a row for an agent of a type with no MCP command.

Repeating the action for the same agent and server SHALL show the terminal
already opened for them rather than open another. The action SHALL show and
start its terminal whatever the agent's activation mode.

That terminal SHALL be recognisable as a handover terminal after a restart. One
left by an earlier run SHALL NOT be restored, and one left by a window that
closed SHALL be removed when that workspace's window opens again, before
anything starts.

#### Scenario: Delegating for a server needing authentication

- **WHEN** the user invokes the action on a row reporting that the server
  needs authentication
- **THEN** a terminal opens in the agent's working directory with that agent
  type's MCP management command entered, and the agent's ACP session is
  unaffected

#### Scenario: A type with a per-server command uses it

- **WHEN** the user invokes the action on a row of an agent whose type offers
  a command addressing one named server
- **THEN** the terminal receives that command with the row's server named,
  rather than an interactive flow the user must navigate

#### Scenario: The section catches up afterwards

- **WHEN** the terminal opened by the action exits
- **THEN** a probe runs and the section shows its result

#### Scenario: A connected server offers no action

- **WHEN** a row reports its server as connected
- **THEN** that row offers no delegated action

#### Scenario: Repeating the action reuses the terminal

- **WHEN** the user invokes the action twice on the same server's row
- **THEN** one terminal exists for that agent and server, and the second
  invocation shows it

#### Scenario: A passive agent's terminal still runs

- **WHEN** the user invokes the action on a row of a passive agent that is not
  running
- **THEN** the terminal is shown and its flow starts

#### Scenario: A leftover terminal is not restored

- **WHEN** Knot starts with a saved handover terminal from an earlier run
- **THEN** the terminal is not restored, and the agent and its other companions
  are

#### Scenario: A closed window's terminal is removed when it reopens

- **WHEN** a workspace window opens with a handover terminal in its workspace
- **THEN** the terminal is removed before any session starts

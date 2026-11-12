# Spec Delta

## MODIFIED Requirements

### Requirement: The collapsed section names what needs attention

Collapsed, the section's header SHALL say how many of the agent's servers
need attention — that is, are in the needs-authentication or failed state and
have not been dismissed by the user — and SHALL name them, up to a bounded
number, with a remainder counting those the names do not cover. When nothing needs attention, the header SHALL say
so in terms of the servers it knows about rather than showing a bare count or
an empty label. Expanded, the header SHALL count the rows listed below it.

#### Scenario: Something needs attention

- **WHEN** the section is collapsed and two of the agent's four servers need
  authentication
- **THEN** the header names those two servers as needing attention

#### Scenario: Nothing needs attention

- **WHEN** the section is collapsed and every known server is connected
- **THEN** the header says every server is connected, and names none

#### Scenario: Expanded counts

- **WHEN** the section is expanded
- **THEN** the header counts the rows shown

#### Scenario: A dismissed server does not need attention

- **WHEN** the section is collapsed and two of the agent's servers need
  authentication, one of which the user has dismissed
- **THEN** the header names only the other as needing attention

## ADDED Requirements

### Requirement: A server the user does not use can be dismissed

An agent's account can offer servers the user never set up, each reported as
needing authentication. The user SHALL be able to dismiss an agent's own
server that needs attention, and to bring a dismissed server back. Dismissal
SHALL be kept by server name as a preference, across agents and launches. A
dismissed server that is not connected SHALL be listed after the others under
a heading saying it is not connected, drawn without its fix action. Knot's own
server SHALL NOT be dismissable.

#### Scenario: Dismissing an unused connector

- **WHEN** the user dismisses a server that needs authentication
- **THEN** it moves below the other servers under the not-connected heading,
  and it no longer counts as needing attention

#### Scenario: Bringing a server back

- **WHEN** the user brings a dismissed server back
- **THEN** it is listed with the others again, and counts as needing
  attention if its state does

#### Scenario: A dismissed server that connects

- **WHEN** a server the user dismissed is reported as connected
- **THEN** it is listed with the others, as connected, rather than under the
  not-connected heading

#### Scenario: Dismissal outlives the window

- **WHEN** the user dismisses a server and Knot is restarted
- **THEN** that server is still dismissed, in every agent that reports it

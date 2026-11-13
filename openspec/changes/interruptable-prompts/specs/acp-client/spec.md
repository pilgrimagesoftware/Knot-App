# Spec Delta

## ADDED Requirements

### Requirement: Steering a running turn

The system SHALL record that an agent supports steering when its `initialize`
response carries `_meta.steering.supported` true, and only then SHALL send
`_session/steering` to hand a prompt to the session's running turn, asking for
`promptRequired` when no turn is running. The system SHALL report the outcome
as injected, started a new turn, or prompt required.

#### Scenario: An agent that advertises steering

- **WHEN** an agent's `initialize` response carries
  `_meta.steering.supported: true`
- **THEN** the client records that it supports steering

#### Scenario: Steering into a running turn

- **WHEN** the client steers a prompt while the agent's turn is running
- **THEN** it sends `_session/steering` with
  `_meta.steering.idleBehavior: "promptRequired"`, and reports the outcome as
  injected

#### Scenario: No running turn to steer

- **WHEN** the agent answers a steer with `promptRequired`
- **THEN** the client reports that the prompt must be sent as a new turn

#### Scenario: An agent without steering

- **WHEN** the agent's `initialize` response does not advertise steering
- **THEN** the client refuses to steer without sending anything

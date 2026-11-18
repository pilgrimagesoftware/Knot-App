# Spec Delta

## ADDED Requirements

### Requirement: Closing a workspace window stops its agents

Closing a workspace window SHALL release its view, stopping every adapter
and terminal session it started and every prompt it would send them.

#### Scenario: A scheduled window is closed

- **WHEN** a scheduled run opens a workspace window and that window is closed
- **THEN** the window's view is released and its sessions are torn down

### Requirement: An agent runs one adapter at a time

The app SHALL NOT start an adapter for an agent while another adapter for
that agent is connecting or connected, in any window. An adapter that has
been stopped or has failed SHALL NOT block the next one.

#### Scenario: A second start for a running agent

- **WHEN** an agent's adapter is connecting or connected and another start
  is requested for it
- **THEN** no second adapter is started

#### Scenario: A restart

- **WHEN** an agent is restarted, stopping its adapter and starting another
- **THEN** the new adapter starts

### Requirement: A window opened by a schedule is listed

A workspace window opened by a scheduled start SHALL be listed in the Window
menu as soon as it opens, though it does not take focus.

#### Scenario: A run opens a window in the background

- **WHEN** a scheduled run opens its workspace's window without focus
- **THEN** the Window menu lists that workspace

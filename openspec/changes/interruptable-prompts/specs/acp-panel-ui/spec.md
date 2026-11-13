# Spec Delta

## ADDED Requirements

### Requirement: Steer a queued prompt into the running turn

For an agent that supports steering, while a turn is running, each queued
prompt SHALL offer Steer now, which hands that prompt to the running turn
instead of waiting for it to end. A prompt the agent accepts SHALL leave the
queue and read as a prompt in the running turn. A prompt the agent hands back
for lack of a running turn SHALL return to the head of the queue. A steer that
fails SHALL leave the prompt queued with the failure shown. Send SHALL keep
queueing.

#### Scenario: Correcting course mid-turn

- **WHEN** a Claude agent is working, the user queues "also check the
  tests", and chooses Steer now on it
- **THEN** the prompt leaves the queue, appears in the conversation, and the
  agent takes it up in the turn it is running

#### Scenario: The turn ended first

- **WHEN** the user chooses Steer now just as the turn ends, and the agent
  answers that no turn is running
- **THEN** the prompt is back at the head of the queue and is delivered as
  the next turn

#### Scenario: An agent without steering

- **WHEN** a Gemini agent is working and the user queues a prompt
- **THEN** no Steer now is offered, and the prompt is delivered when the turn
  ends

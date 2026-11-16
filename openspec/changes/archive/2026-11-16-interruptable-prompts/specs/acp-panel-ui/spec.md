# Spec Delta

## MODIFIED Requirements

### Requirement: Prompt queue
The system SHALL queue prompts delivered during a response and deliver them to
the agent in the order they were enqueued, one turn at a time, until the queue
empties. A queued prompt SHALL be visible in the conversation, clearly marked
as waiting until it is delivered, and SHALL read as a normal prompt once
delivered. Delivery SHALL resume on each turn end; a prompt whose delivery
fails SHALL be reported under that prompt, and the queue SHALL continue with
the remaining prompts. A prompt steered into the running turn is the exception: it leaves the queue when the agent takes it, and returns to the head when handed back.

#### Scenario: Prompts are delivered in order
- **WHEN** the user enqueues "first" and then "second" while a response is in
  progress
- **THEN** "first" is delivered when the current turn ends and "second" is
  delivered when the turn "first" started has ended

#### Scenario: A queued prompt is marked until delivered
- **WHEN** a prompt is enqueued during a response
- **THEN** the conversation shows it as waiting, and once its turn starts the
  same message reads as a delivered prompt

#### Scenario: A failed queued delivery continues the queue
- **WHEN** a queued prompt's delivery fails
- **THEN** the failure is reported under that prompt, the turn ends, and the
  next queued prompt is delivered

#### Scenario: A clean queue delivers immediately
- **WHEN** the user sends while no prompt is queued and no response is in
  progress
- **THEN** the prompt is delivered immediately and no queue entry is created

#### Scenario: A steered prompt leaves the queue mid-turn
- **WHEN** "first" and "second" are queued during a response and the agent
  takes "second" into its running turn with Steer now
- **THEN** "second" leaves the queue and reads as a prompt in that turn, and
  "first" is still delivered when the turn ends

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

### Requirement: Steering waits for a pending permission

Steer now SHALL NOT be offered, and no prompt SHALL be steered, while the
agent has a permission request pending, as for any other prompt under
"Permission gate is independent of the queue".

#### Scenario: A permission is pending mid-turn

- **WHEN** an agent that supports steering is waiting on a permission request
  and the user has a prompt queued
- **THEN** no Steer now is offered and nothing is sent until the request is
  answered

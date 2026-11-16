# Spec Delta

## ADDED Requirements

### Requirement: Prompts wait for a session limit to lift

A prompt to a Panel-mode agent whose session is stopped by its usage limit SHALL
be queued rather than delivered, and the queue SHALL deliver nothing to that
agent until the limit's reset time has passed, after which it SHALL drain as
usual without the user acting. A reset time SHALL be read as the next occurrence
of its time of day in the time zone it names after the refusal; a reset that
cannot be read so SHALL hold the queue until the user overrides it.

#### Scenario: A prompt sent during a session limit

- **WHEN** the user sends a prompt to an agent stopped by "resets 5:20pm
  (America/Los_Angeles)"
- **THEN** the prompt is queued, and is delivered on its own once 5:20 pm in
  Los Angeles has passed

#### Scenario: A reset time that cannot be read

- **WHEN** the limit gives no reset time Knot can read
- **THEN** queued prompts wait until the user sends one now

### Requirement: A held prompt can be sent now

While a session limit holds the queue, each waiting row SHALL say it is waiting
for the session limit and SHALL offer Send now. Send now SHALL move that entry to
the head of the queue and deliver it next, past the limit; the rest SHALL keep
waiting.

#### Scenario: Sending one prompt now

- **WHEN** two prompts are held and the user chooses Send now on the second
- **THEN** the second is delivered next and the first keeps waiting

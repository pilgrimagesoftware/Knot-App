# Spec Delta

## ADDED Requirements

### Requirement: A named usage limit is a session limit

The system SHALL recognise a usage-limit notice by its stable lead, "You've
hit your" or "You've reached your", followed by a short name ending in
"limit", whatever the limit is called, and SHALL mark the agent with a session
limit for it as for the older "session limit" wording. A limit that does not
stop the session SHALL NOT be one.

#### Scenario: A turn refused at the 5-hour limit

- **WHEN** a Panel-mode agent's prompt is answered with "You've hit your 5-hour
  limit · resets 5:20pm (America/Los_Angeles)"
- **THEN** the agent shows "Session limit · resets 5:20pm
  (America/Los_Angeles)" with a red dot, as for the older "session limit"
  wording

#### Scenario: Weekly and per-model limits

- **WHEN** an agent is refused with "You've hit your weekly limit" or "You've
  hit your Opus limit", or a terminal agent prints either at the start of a
  line
- **THEN** the agent shows "Session limit"

#### Scenario: The other stable lead

- **WHEN** an agent is refused with "You've reached your Fable limit."
- **THEN** the agent shows "Session limit"

#### Scenario: Fast mode's limit is not a session limit

- **WHEN** an agent prints or is answered with "You've hit your fast limit ·
  resets in 4m"
- **THEN** its status is unaffected, because the agent carries on at normal
  speed

#### Scenario: A lead that names no limit

- **WHEN** an agent is answered with "You've hit your team's shared budget."
- **THEN** its status is unaffected

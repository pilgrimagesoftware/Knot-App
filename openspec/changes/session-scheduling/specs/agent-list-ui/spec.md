# Spec Delta

## ADDED Requirements

### Requirement: A scheduled agent shows its next run

An agent row SHALL show a clock-labelled detail line with the agent's next
run time when it has a schedule, and "missed" with the time for an hour after
a missed run. The compact sidebar SHALL show the same text in the row's
tooltip. An agent without a schedule shows neither.

#### Scenario: A scheduled agent's row

- **WHEN** an agent is scheduled every weekday at 09:00 and it is Friday
  evening
- **THEN** its row shows a clock line naming Monday 09:00

### Requirement: An idle countdown shows near its end

An agent row SHALL show a timer-labelled detail line while an idle shutdown
is near: once the time left is the smaller of 10 minutes or half the idle
period. It SHALL say how long the agent has been idle, what will happen, and
in how long. The workspace header SHALL show a timer icon with that text as
its tooltip while the selected agent's line is shown.

#### Scenario: Early in an idle period

- **WHEN** an agent with a 60-minute idle period has been idle for 20 minutes
- **THEN** its row shows no timer line

#### Scenario: Near the end

- **WHEN** the same agent has been idle for 52 minutes
- **THEN** its row shows a timer line saying it has been idle 52 minutes and
  will be deactivated in 8

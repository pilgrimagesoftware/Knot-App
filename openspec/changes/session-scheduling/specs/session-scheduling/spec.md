# Spec Delta

## ADDED Requirements

### Requirement: An agent can be started on a schedule

An agent that is not a shell agent and has a startup prompt MAY carry a
schedule. When a run comes due while Knot is running, Knot SHALL open or
raise the agent's workspace window and start the agent in a new
conversation, so that its startup prompt runs. An agent without a startup
prompt SHALL NOT be given a schedule.

#### Scenario: A daily run

- **WHEN** an agent is scheduled every day at 09:00 and it is 09:00
- **THEN** the agent is started in a new conversation and its startup prompt
  is sent to it

#### Scenario: The workspace window is closed

- **WHEN** a run comes due and the agent's workspace window is not open
- **THEN** the window opens and the agent starts in it

### Requirement: A schedule is a preset or a cron expression

A schedule SHALL be either a preset — every day, every weekday, or chosen
days, at a time; or every N hours on the hour — or a standard five-field cron
expression. Both SHALL be evaluated in the Mac's local time zone. A local
time skipped by a clock change SHALL run at the first valid time after it,
and a time that occurs twice SHALL run once.

#### Scenario: A cron expression

- **WHEN** an agent is scheduled `30 8 * * 1-5`
- **THEN** it runs at 08:30 local time on weekdays only

#### Scenario: A skipped hour

- **WHEN** a run is scheduled at 02:30 on a day the clocks go forward from
  02:00 to 03:00
- **THEN** it runs once, at 03:00

### Requirement: A scheduled run never interrupts a turn

A run that comes due while the agent has a turn in progress SHALL be skipped
and recorded as skipped. The next occurrence is unaffected.

#### Scenario: Busy at run time

- **WHEN** a run comes due while the agent is working
- **THEN** the turn continues, no new conversation is started, and the run
  is recorded as skipped

### Requirement: Missed runs run at most once, and only when recent

A run due while Knot was not running or the Mac was asleep SHALL run once
when Knot next sees it, if it is at most 10 minutes late. An older run SHALL
be recorded as missed and not run. Knot SHALL never run more than one missed
run of an agent, and SHALL never run the same scheduled run twice.

#### Scenario: Waking shortly after

- **WHEN** the Mac wakes at 09:04 and a run was due at 09:00
- **THEN** that run runs once

#### Scenario: Waking hours later

- **WHEN** Knot launches at 13:00 and runs were due at 09:00 and 12:00
- **THEN** neither runs, the 12:00 run is shown as missed, and the next run
  is the following occurrence

### Requirement: An agent can be stopped after it has been idle

Knot SHALL deactivate or reset (restart in a new conversation) an agent that
has been idle for its idle period. The period and action come from the
agent's own idle setting, or from the settings default when the agent
inherits it. Both are off unless set. Shell agents are not affected.

#### Scenario: Idle deactivation

- **WHEN** an agent's idle policy is "deactivate after 30 minutes" and it has
  been idle for 30 minutes
- **THEN** it is deactivated, as if the user chose Deactivate

#### Scenario: An agent overrides the default

- **WHEN** the settings default is to reset after an hour and an agent's own
  idle setting is off
- **THEN** that agent is never stopped for idling

### Requirement: Idle means nothing is happening and nobody is looking

An agent SHALL count as idle only while it has no turn in progress, no
running subagents, no queued prompt, no pending permission request, and no
running background process under its session besides its own adapter or
shell. Typing, sending, or scrolling in its conversation or terminal SHALL
restart the idle period. Awaiting input counts as idle.

#### Scenario: Scrolling back counts as activity

- **WHEN** the user scrolls an agent's conversation 25 minutes into a
  30-minute idle period
- **THEN** the idle period starts again from that moment

#### Scenario: A background process keeps it awake

- **WHEN** an agent's turn has ended but a test run it started is still
  running
- **THEN** the agent is not idle until that process ends

### Requirement: An idle reset does not run the startup prompt

A reset caused by idling SHALL start a new conversation without sending the
agent's startup prompt. Only a scheduled run or a start the user makes SHALL
send it.

#### Scenario: A scripted agent goes idle

- **WHEN** an agent with a startup prompt and an idle reset finishes its
  script and goes idle for its period
- **THEN** it is reset into a new conversation, and its startup prompt is not
  sent again

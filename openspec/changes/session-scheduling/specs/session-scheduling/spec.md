# Spec Delta

## ADDED Requirements

### Requirement: An agent can be started on a schedule

An agent that is not a shell agent and has a startup prompt MAY carry a
schedule. When a run comes due while Knot is running, Knot SHALL open or
raise the agent's workspace window and start the agent in a new
conversation, activating it first if it was deactivated, so that its
startup prompt runs. An agent without a startup prompt SHALL NOT be given a
schedule.

#### Scenario: A daily run

- **WHEN** an agent is scheduled every day at 09:00 and it is 09:00
- **THEN** the agent is started in a new conversation and its startup prompt
  is sent to it

#### Scenario: A run comes due for a deactivated agent

- **WHEN** a run comes due for an agent the user stopped, or that was
  deactivated for idling
- **THEN** it is activated and started in a new conversation, and its startup
  prompt is sent

#### Scenario: The workspace window is closed

- **WHEN** a run comes due and the agent's workspace window is not open
- **THEN** the window opens and the agent starts in it, without taking focus
  from another app the user is working in

### Requirement: A schedule is a preset or a cron expression

A schedule SHALL be either a preset — every day, every weekday, or chosen
days, at a time; or every N hours, at the hours from midnight that are
multiples of N, so the last gap of a day is shorter when N does not divide
24 — or a standard five-field cron expression. Both SHALL be evaluated in the Mac's local time zone. A local
time skipped by a clock change SHALL run at the first valid time after it,
and a time that occurs twice SHALL run once.

#### Scenario: A cron expression

- **WHEN** an agent is scheduled `30 8 * * 1-5`
- **THEN** it runs at 08:30 local time on weekdays only

#### Scenario: A skipped hour

- **WHEN** a run is scheduled at 02:30 on a day the clocks go forward from
  02:00 to 03:00
- **THEN** it runs once, at 03:00

### Requirement: A scheduled run never discards the agent's work

A run SHALL be skipped, and recorded as skipped with the reason, when the
agent is busy: it has a turn in progress, a pending permission request, a
queued prompt, or an unanswered message it holds. The next occurrence is
unaffected.

#### Scenario: Busy at run time

- **WHEN** a run comes due while the agent is working
- **THEN** the turn continues, no new conversation is started, and the run
  is recorded as skipped

#### Scenario: An unanswered brief

- **WHEN** a run comes due for an idle agent that holds an unanswered message
- **THEN** the run is skipped because the agent owes a reply, and the message
  stays as it was

### Requirement: Missed runs run at most once, and only when recent

A run due while Knot was not running or the Mac was asleep SHALL run once
when Knot next sees it, if it is at most 10 minutes late. An older run SHALL
be recorded as missed and not run. Knot SHALL never run more than one missed
run of an agent, and SHALL never handle the same scheduled run twice, including
across a relaunch: a run once skipped or missed is not started later.

#### Scenario: Waking shortly after

- **WHEN** the Mac wakes at 09:04 and a run was due at 09:00
- **THEN** that run runs once

#### Scenario: Waking hours later

- **WHEN** Knot launches at 13:00 and runs were due at 09:00 and 12:00
- **THEN** neither runs, the 12:00 run is shown as missed, and the next run
  is the following occurrence

#### Scenario: A schedule set after its time waits for the next run

- **WHEN** at 09:05 the user gives an idle agent the schedule "every day at
  09:00", or changes its schedule to it
- **THEN** the agent is not started now; its first run is 09:00 the next day,
  since a run that fell due before the schedule existed was never pending

#### Scenario: Relaunching just after a run

- **WHEN** the 09:00 run has run, and the user quits and relaunches Knot at
  09:05
- **THEN** the 09:00 run does not run again

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
running subagents, no queued prompt, no pending permission request, no
unanswered message it holds, and no running background process its latest
turn started. Typing, sending, or scrolling in
its conversation or terminal SHALL restart the idle period.

#### Scenario: Scrolling back counts as activity

- **WHEN** the user scrolls an agent's conversation 25 minutes into a
  30-minute idle period
- **THEN** the idle period starts again from that moment

#### Scenario: An unanswered brief keeps it awake

- **WHEN** an agent's turn has ended but it holds a message it has not
  answered
- **THEN** it is not idle, and is neither deactivated nor reset, until it
  answers

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

### Requirement: An agent that asked the user a question can go idle

An agent whose turn ended asking the user something SHALL count as idle,
since nobody has answered. An agent waiting on a pending permission request
SHALL NOT, because resetting or deactivating it would discard the action it
is waiting to take.

#### Scenario: A question left unanswered

- **WHEN** an agent's turn ended with a question to the user and nobody
  answers for its idle period
- **THEN** it is stopped for idling

#### Scenario: A permission request left pending

- **WHEN** an agent has waited on a permission request for longer than its
  idle period
- **THEN** it is not stopped for idling

### Requirement: An idle reset happens once per stretch of idleness

After an idle reset, the agent SHALL NOT be reset for idling again until it
has been busy or the user has acted in its pane. A fresh conversation nobody
has used is idle by definition; resetting it every period would start a new
adapter each time for nothing.

#### Scenario: An untouched fresh conversation

- **WHEN** an agent was reset for idling and nothing has happened in its new
  conversation for several of its idle periods
- **THEN** it is not reset again

#### Scenario: Used, then idle again

- **WHEN** the user sends that agent a prompt, its turn ends, and it then idles
  for its period
- **THEN** it is reset again


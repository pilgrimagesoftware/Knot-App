//! Idle shutdown (#404): noticing that an agent has done nothing and nobody
//! has looked at it for its idle period, then deactivating or resetting it.
//!
//! Contract: `openspec/changes/session-scheduling` - "An agent can be
//! stopped after it has been idle", "Idle means nothing is happening and
//! nobody is looking", "An agent that asked the user a question can go idle"
//! and "An idle reset does not run the startup prompt".
//!
//! An agent's idle period starts at the latest of: when its session was first
//! tracked, its last turn ending (`Agent::idle_since`), the last time it was
//! seen busy, and the last user activity in its pane. Busy is checked every
//! [`IDLE_CHECK_INTERVAL`]: a turn, a pending permission, a queued prompt, an
//! unanswered message, a running subagent, or a background process the
//! agent's latest turn started. That last rule is an approximation: once the
//! agent has had a turn, a descendant counts only if it started during or
//! after that turn, so helpers from earlier turns (a stdio MCP server started
//! on first use) stop counting when the next turn begins. Before any turn,
//! the processes the session started with are its baseline and only later
//! ones count. A helper started lazily during the latest turn still counts
//! until the next one; timing alone cannot tell it from a test run.
//!
//! What it owns: one [`AgentIdle`] per agent, pruned in `teardown_session`.

use std::collections::BTreeSet;
use std::time::{Duration, Instant, SystemTime};

use gpui_kit::Context;
use knot_core::schedule::{IdleAction, IdleRule};
use uuid::Uuid;

use crate::consts::{IDLE_CHECK_INTERVAL, IDLE_COUNTDOWN_WINDOW};
use crate::workspace_window::WorkspaceWindow;

/// One agent's idle tracking. Runtime only.
#[derive(Debug, Clone)]
pub(crate) struct AgentIdle {
    /// When tracking began for the current session.
    tracked_since:           SystemTime,
    last_user_activity:      Option<SystemTime>,
    last_busy_seen:          Option<SystemTime>,
    /// The session's root pid and the descendants it started with.
    process_baseline:        Option<(u32, BTreeSet<u32>)>,
    /// When the agent's latest turn began (to the precision of the checks),
    /// or `None` before its first turn in this session.
    turn_started_at:         Option<SystemTime>,
    /// Whether the last check saw it working, to notice a turn starting.
    was_working:             bool,
    /// When the last check ran.
    last_checked:            Option<SystemTime>,
    /// A descendant outside the baseline was running at the last sample.
    process_active:          bool,
    /// Set by an idle reset: the next session starts without the startup
    /// prompt. Read once, by the session start.
    withhold_startup_prompt: bool,
    /// Set by an idle reset and cleared by the next activity, so a fresh,
    /// untouched conversation is not reset again every period.
    reset_awaits_activity:   bool,
    /// What the sidebar shows, from the last check.
    pub(crate) countdown:    Option<IdleCountdown>,
}

impl AgentIdle {
    fn new(now: SystemTime) -> Self {
        Self { tracked_since:           now,
               last_user_activity:      None,
               last_busy_seen:          None,
               process_baseline:        None,
               turn_started_at:         None,
               was_working:             false,
               last_checked:            None,
               process_active:          false,
               withhold_startup_prompt: false,
               reset_awaits_activity:   false,
               countdown:               None, }
    }
}

/// How long an agent has been idle and what will happen, shown near the end
/// of the period. Minute-granular, so it changes - and repaints - once a
/// minute rather than every check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IdleCountdown {
    pub(crate) idle_minutes: u64,
    pub(crate) left_minutes: u64,
    pub(crate) action:       IdleAction,
}

/// The pure part: given whether the agent is busy now, when its idle period
/// started and its rule, how far into the period it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdleVerdict {
    Busy,
    /// Idle, with time still to go. `countdown` is set once it is close.
    Idling {
        countdown: Option<IdleCountdown>,
    },
    Due,
}

/// When the idle period began: the latest of the marks, never before
/// tracking began.
pub(crate) fn idle_start(tracked_since: SystemTime, marks: &[Option<SystemTime>]) -> SystemTime {
    marks.iter()
         .flatten()
         .copied()
         .fold(tracked_since, SystemTime::max)
}

pub(crate) fn verdict(busy: bool, started: SystemTime, rule: IdleRule, now: SystemTime)
                      -> IdleVerdict {
    if busy {
        return IdleVerdict::Busy;
    }
    let idle = now.duration_since(started).unwrap_or_default();
    let period = Duration::from_secs(u64::from(rule.after_minutes()) * 60);
    if idle >= period {
        return IdleVerdict::Due;
    }
    let left = period - idle;
    let window = IDLE_COUNTDOWN_WINDOW.min(period / 2);
    let countdown = (left <= window).then(|| IdleCountdown { idle_minutes: idle.as_secs() / 60,
                                                             left_minutes: left.as_secs()
                                                                               .div_ceil(60),
                                                             action:       rule.action(), });
    IdleVerdict::Idling { countdown }
}

impl WorkspaceWindow {
    /// The user typed, clicked or scrolled in `id`'s pane.
    pub(crate) fn note_user_activity(&mut self, id: Uuid) {
        let now = SystemTime::now();
        let entry = self.agent_idle
                        .entry(id)
                        .or_insert_with(|| AgentIdle::new(now));
        entry.last_user_activity = Some(now);
        entry.reset_awaits_activity = false;
    }

    /// A process sample landed for `id`, whose session root is `root`, at
    /// `now`: each descendant's pid and how long it has run. See the module
    /// doc for which count as activity.
    pub(crate) fn note_process_sample(&mut self, id: Uuid, root: u32,
                                      processes: &[(u32, Duration)], now: SystemTime) {
        let entry = self.agent_idle
                        .entry(id)
                        .or_insert_with(|| AgentIdle::new(now));
        let pids: BTreeSet<u32> = processes.iter().map(|(pid, _)| *pid).collect();
        let same_session = entry.process_baseline
                                .as_ref()
                                .is_some_and(|(baseline_root, _)| *baseline_root == root);
        if !same_session {
            // A new session: what it starts with is its infrastructure, and
            // the old session's turns say nothing about it.
            entry.process_baseline = Some((root, pids));
            entry.turn_started_at = None;
            entry.process_active = false;
            return;
        }
        entry.process_active = match entry.turn_started_at {
            Some(turn) => processes.iter().any(|(_, elapsed)| {
                                              now.checked_sub(*elapsed)
                                                 .is_some_and(|started| started >= turn)
                                          }),
            None => {
                let baseline = entry.process_baseline
                                    .as_ref()
                                    .map(|(_, baseline)| baseline.clone())
                                    .unwrap_or_default();
                pids.iter().any(|pid| !baseline.contains(pid))
            }
        };
    }

    /// Whether the next session `id` starts should go without its startup
    /// prompt, consuming the answer.
    pub(crate) fn take_withheld_startup_prompt(&mut self, id: Uuid) -> bool {
        self.agent_idle
            .get_mut(&id)
            .is_some_and(|entry| std::mem::take(&mut entry.withhold_startup_prompt))
    }

    /// Agents an idle policy applies to, whose processes need sampling.
    pub(crate) fn idle_tracked_agents(&self, cx: &gpui_kit::App) -> Vec<Uuid> {
        let default = crate::settings_global::read(cx).idle_default;
        let store = self.store.lock();
        // The free function, under this one lock: the method takes the store
        // lock itself, and `parking_lot` locks are not reentrant.
        super::workspace_agent_ids(&store, self.workspace_id).into_iter()
                                                             .filter(|id| {
                                                                 store.agent(*id)
                                                                      .is_some_and(|agent| {
                                                                          agent.activated
                                    && !agent.is_shell()
                                    && agent.idle_policy.effective(default).is_some()
                                                                      })
                                                             })
                                                             .collect()
    }

    /// One idle check, at most every [`IDLE_CHECK_INTERVAL`]. Answers whether
    /// anything the sidebar shows changed.
    pub(crate) fn idle_tick(&mut self, cx: &mut Context<Self>) -> bool {
        if self.idle_checked_at
               .is_some_and(|at| at.elapsed() < IDLE_CHECK_INTERVAL)
        {
            return false;
        }
        self.idle_checked_at = Some(Instant::now());
        self.check_idle_at(SystemTime::now(), cx)
    }

    /// The check itself, at `now` (a parameter so the tests can choose it).
    pub(crate) fn check_idle_at(&mut self, now: SystemTime, cx: &mut Context<Self>) -> bool {
        let default = crate::settings_global::read(cx).idle_default;
        let mut changed = false;
        let mut due = Vec::new();
        for id in self.idle_tracked_agents(cx) {
            let Some((rule, idle_since)) = ({
                let store = self.store.lock();
                store.agent(id).and_then(|agent| {
                                   Some((agent.idle_policy.effective(default)?, agent.idle_since))
                               })
            })
            else {
                continue;
            };
            let working = self.store
                              .lock()
                              .agent(id)
                              .is_some_and(|agent| agent.state == knot_agents::AgentState::Running);
            let busy = self.idle_busy(id);
            let entry = self.agent_idle
                            .entry(id)
                            .or_insert_with(|| AgentIdle::new(now));
            // A turn began since the last check: date it from that check, the
            // latest moment it was known not to be working.
            if working && !entry.was_working {
                entry.turn_started_at = Some(entry.last_checked.unwrap_or(now));
            }
            entry.was_working = working;
            entry.last_checked = Some(now);
            if busy {
                entry.last_busy_seen = Some(now);
                entry.reset_awaits_activity = false;
            }
            let started = idle_start(entry.tracked_since,
                                     &[idle_since, entry.last_busy_seen, entry.last_user_activity]);
            let verdict = verdict(busy || entry.reset_awaits_activity, started, rule, now);
            let countdown = match verdict {
                IdleVerdict::Idling { countdown } => countdown,
                IdleVerdict::Busy | IdleVerdict::Due => None,
            };
            if entry.countdown != countdown {
                entry.countdown = countdown;
                changed = true;
            }
            if verdict == IdleVerdict::Due {
                due.push((id, rule.action()));
            }
        }
        for (id, action) in due {
            self.stop_for_idling(id, action, now, cx);
            changed = true;
        }
        changed
    }

    /// Busy for idle purposes: everything a scheduled run treats as busy,
    /// plus a turn under way, a running subagent and a new background
    /// process.
    fn idle_busy(&self, id: Uuid) -> bool {
        let working = self.store
                          .lock()
                          .agent(id)
                          .is_some_and(|agent| agent.state == knot_agents::AgentState::Running);
        let subagents = self.subagents.lock().running_count(id) > 0;
        let processes = self.agent_idle
                            .get(&id)
                            .is_some_and(|entry| entry.process_active);
        working || subagents || processes || self.scheduled_run_blocker(id).is_some()
    }

    fn stop_for_idling(&mut self, id: Uuid, action: IdleAction, now: SystemTime,
                       cx: &mut Context<Self>) {
        match action {
            IdleAction::Deactivate => {
                self.deactivate_agent(id);
            }
            IdleAction::Reset => {
                // A fresh entry for the new session, which starts without its
                // startup prompt and is not reset again until it is used.
                let mut entry = AgentIdle::new(now);
                entry.withhold_startup_prompt = true;
                entry.reset_awaits_activity = true;
                self.agent_idle.insert(id, entry);
                self.restart_agents(&[id], false, cx);
            }
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests;

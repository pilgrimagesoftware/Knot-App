//! The app-level ticker that starts agents on their schedules (#404).
//!
//! Contract: `openspec/changes/session-scheduling` - "An agent can be started
//! on a schedule", "A scheduled run never discards the agent's work" and
//! "Missed runs run at most once, and only when recent".
//!
//! Window-independent by necessity: sessions live in workspace windows, and a
//! run must come due with the window closed. Each tick reads every scheduled
//! agent from the shared store, asks `knot_core::schedule::decide` what to do,
//! records the run as handled *before* acting - so no crash or slow tick can
//! handle it twice, and a relaunch reads it back - and only then starts it,
//! through the owning window (`WorkspaceWindow::start_scheduled_run`), opened
//! without taking focus from another app.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::{App, Global};
use jiff::{Timestamp, Zoned};
use knot_core::schedule::{RunDecision, Schedule, SkipReason, decide};
use parking_lot::Mutex;
use uuid::Uuid;

use crate::consts::SCHEDULE_TICK_INTERVAL;
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::scheduling::owes_a_reply;

/// How an agent's latest scheduled run went, for its sidebar line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScheduleOutcome {
    Started(Timestamp),
    Skipped(Timestamp, SkipReason),
    Missed(Timestamp),
}

/// The latest outcome per agent, since launch. Runtime only: what the sidebar
/// says about a missed run is worth an hour, not a relaunch.
#[derive(Default)]
pub(crate) struct ScheduleOutcomes(pub(crate) BTreeMap<Uuid, ScheduleOutcome>);

impl Global for ScheduleOutcomes {}

/// Starts the ticker. Called once, at launch.
pub(crate) fn install(store: Arc<Mutex<knot_agents::AgentStore>>,
                      messages: Arc<Mutex<knot_messaging::MessageStore>>, cx: &mut App) {
    cx.set_global(ScheduleOutcomes::default());
    cx.spawn(async move |cx| {
          // For the app's lifetime, like the adapter update check.
          loop {
              cx.background_executor().timer(SCHEDULE_TICK_INTERVAL).await;
              cx.update(|cx| tick(&store, &messages, &Zoned::now(), cx));
          }
      })
      .detach();
}

/// One scheduled agent, snapshotted under one store lock.
struct Scheduled {
    workspace:    Uuid,
    agent:        Uuid,
    schedule:     Schedule,
    last_handled: Option<Timestamp>,
}

/// One pass over every scheduled agent, at `now` (the clock is a parameter
/// so the tests can choose it).
pub(crate) fn tick(store: &Arc<Mutex<knot_agents::AgentStore>>,
                   messages: &Arc<Mutex<knot_messaging::MessageStore>>, now: &Zoned,
                   cx: &mut App) {
    // Snapshotted first, in its own statement: a guard in a `for` loop's
    // iterator expression lives until the loop ends, and the body takes the
    // store lock again - a deadlock, since `parking_lot` locks are not
    // reentrant.
    let scheduled_agents = scheduled_agents(&store.lock());
    for scheduled in scheduled_agents {
        let key = WindowKey::Workspace(scheduled.workspace);
        let busy = match WindowRegistry::workspace_view(key, cx) {
            Some(view) => view.read(cx).scheduled_run_blocker(scheduled.agent),
            // No window, no session: only a message it owes a reply to can
            // make it busy.
            None => {
                owes_a_reply(&store.lock(), &messages.lock(), scheduled.agent)
                    .then_some(SkipReason::OwesReply)
            }
        };
        let decision = decide(&scheduled.schedule, scheduled.last_handled, now, busy);
        let Some(handled) = decision.handled()
        else {
            continue;
        };
        record_handled(store, &scheduled, handled, cx);
        let outcome = match &decision {
            RunDecision::Start { .. } => ScheduleOutcome::Started(handled),
            RunDecision::Skip { reason, .. } => ScheduleOutcome::Skipped(handled, *reason),
            RunDecision::Missed { .. } => ScheduleOutcome::Missed(handled),
            RunDecision::NotDue => continue,
        };
        cx.global_mut::<ScheduleOutcomes>()
          .0
          .insert(scheduled.agent, outcome);
        if matches!(decision, RunDecision::Start { .. })
           && let Some(view) = WorkspaceWindow::open_for_schedule(Arc::clone(store),
                                                                  Arc::clone(messages),
                                                                  scheduled.workspace,
                                                                  cx)
        {
            view.update(cx, |view, cx| view.start_scheduled_run(scheduled.agent, cx));
            // A window opened without focus never becomes the active one,
            // which is what otherwise rebuilds the Window menu; without this
            // nothing showed that the workspace was open (#456).
            crate::menu_bar::refresh_menu_bar_workspaces(store, cx);
        }
        // The sidebar's clock line changes with every handled run, started
        // or not; nothing else would repaint an idle window for it.
        WindowRegistry::refresh(key, cx);
    }
}

/// Every agent with a runnable schedule and a startup prompt to run, with
/// the run it last handled. Shell agents have neither.
fn scheduled_agents(store: &knot_agents::AgentStore) -> Vec<Scheduled> {
    store.workspaces()
         .iter()
         .flat_map(|workspace| {
             workspace.agent_ids
                      .iter()
                      .map(move |id| (workspace.id, *id))
         })
         .filter_map(|(workspace, id)| {
             let agent = store.agent(id)?;
             let schedule = agent.schedule.clone().filter(Schedule::is_runnable)?;
             if agent.is_shell() || agent.startup_prompt.is_none() {
                 return None;
             }
             let last_handled = store.schedule_run_handled(workspace, id)
                                     .and_then(|seconds| Timestamp::from_second(seconds).ok());
             Some(Scheduled { workspace,
                              agent: id,
                              schedule,
                              last_handled })
         })
         .collect()
}

/// Records the run as handled in the store and persists the UI-state
/// document, before anything acts on it.
fn record_handled(store: &Arc<Mutex<knot_agents::AgentStore>>, scheduled: &Scheduled,
                  handled: Timestamp, cx: &App) {
    // Scoped so the store guard is released before the write.
    let installed = {
        let mut store = store.lock();
        store.record_schedule_run_handled(scheduled.workspace,
                                          scheduled.agent,
                                          handled.as_second());
        crate::settings_global::write(cx, |settings| {
            settings.workspace_ui = store.saved_workspace_ui();
        })
    };
    if let Err(error) = installed.persist_workspace_ui() {
        eprintln!("failed to persist a handled scheduled run: {error}");
    }
}

#[cfg(test)]
mod tests;

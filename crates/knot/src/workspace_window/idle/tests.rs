//! Idle shutdown (#404): the pure verdict, and a real workspace window
//! deciding with a clock the test chooses.
//!
//! The agents use an inert agent type, so no session launches: a reset is
//! observed as the restart the window records and the startup prompt it
//! withholds, not as a new adapter. Process activity and the pane's activity
//! capture are fed in through the same entry points the sampler and the
//! capture listeners call.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use knot_core::schedule::{IdleAction, IdlePolicy, IdleRule};
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use super::{IdleCountdown, IdleVerdict, idle_start, verdict};
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::{PromptOrigin, QueuedPanelPrompt};

const INERT_AGENT_TYPE: &str = "idle-test-inert";

fn mins(n: u64) -> Duration {
    Duration::from_secs(n * 60)
}

fn rule(minutes: u32, action: IdleAction) -> IdleRule {
    IdleRule::new(minutes, action).expect("non-zero")
}

#[test]
fn busy_is_never_idle() {
    let now = SystemTime::now();
    assert_eq!(verdict(true, now - mins(120), rule(30, IdleAction::Deactivate), now),
               IdleVerdict::Busy);
}

#[test]
fn the_countdown_shows_only_near_the_end() {
    let now = SystemTime::now();
    let sixty = rule(60, IdleAction::Deactivate);
    assert_eq!(verdict(false, now - mins(20), sixty, now),
               IdleVerdict::Idling { countdown: None });
    assert_eq!(verdict(false, now - mins(52), sixty, now),
               IdleVerdict::Idling { countdown: Some(IdleCountdown { idle_minutes: 52,
                                                                     left_minutes: 8,
                                                                     action:
                                                                         IdleAction::Deactivate, }), });
    assert_eq!(verdict(false, now - mins(60), sixty, now), IdleVerdict::Due);
}

#[test]
fn a_short_period_counts_down_over_its_second_half() {
    let now = SystemTime::now();
    let ten = rule(10, IdleAction::Reset);
    assert!(matches!(verdict(false, now - mins(4), ten, now),
                     IdleVerdict::Idling { countdown: None }));
    assert!(matches!(verdict(false, now - mins(6), ten, now),
                     IdleVerdict::Idling { countdown: Some(_) }));
}

#[test]
fn the_idle_period_starts_at_the_latest_mark() {
    let start = SystemTime::now() - mins(100);
    let marks = [Some(start + mins(10)), None, Some(start + mins(40))];
    assert_eq!(idle_start(start, &marks), start + mins(40));
    assert_eq!(idle_start(start, &[None]), start);
}

struct Fixture {
    window:   VisualTestContext,
    view:     Entity<WorkspaceWindow>,
    store:    Arc<Mutex<knot_agents::AgentStore>>,
    messages: Arc<Mutex<knot_messaging::MessageStore>>,
    agent:    Uuid,
    sender:   Uuid,
    _dir:     TempDir,
}

fn fixture(policy: IdlePolicy, cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Idle");
    let workspace_id = space.id;
    store.add_workspace(space);
    let mut make = |name: &str| {
        store.create(format!("~/{name}"),
                     knot_agents::CreateOptions { agent_type: Some(INERT_AGENT_TYPE.into()),
                                                  workspace_id: Some(workspace_id),
                                                  ..Default::default() })
    };
    let agent = make("idler");
    let sender = make("sender");
    store.set_activated(agent, true);
    store.set_idle_policy(agent, policy);
    store.set_acp_session_id(agent, "acp-idler".into());
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let view =
        cx.update(|cx| {
              gpui_kit::init(cx);
              WindowRegistry::install(cx);
              crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
              WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), workspace_id, cx);
              WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
          });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    Fixture { window: VisualTestContext::from_window(handle, cx),
              view,
              store,
              messages,
              agent,
              sender,
              _dir: dir }
}

impl Fixture {
    fn check(&mut self, at: SystemTime) {
        self.view.update(&mut self.window, |view, cx| {
                     view.check_idle_at(at, cx);
                 });
    }

    fn activated(&self) -> bool {
        self.store
            .lock()
            .agent(self.agent)
            .expect("exists")
            .activated
    }

    fn restart_token(&self) -> Uuid {
        self.store
            .lock()
            .agent(self.agent)
            .expect("exists")
            .restart_token
    }
}

#[gpui_kit::test]
fn an_agent_idle_for_its_period_is_deactivated(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Deactivate)), cx);
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(29));
    assert!(fixture.activated(), "not yet");
    fixture.check(start + mins(30));
    assert!(!fixture.activated(),
            "deactivated as the context menu would");
}

#[gpui_kit::test]
fn an_agent_set_to_off_ignores_the_settings_default(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Off, cx);
    fixture.window.update(|_, cx| {
                      crate::settings_global::write(cx, |settings| {
                          settings.idle_default = Some(rule(30, IdleAction::Deactivate));
                      });
                  });
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(90));
    assert!(fixture.activated(),
            "an agent whose own setting is off is never stopped");
}

#[gpui_kit::test]
fn a_queued_prompt_keeps_it_awake(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Deactivate)), cx);
    let id = fixture.agent;
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.panel_prompt_queues
                        .entry(id)
                        .or_default()
                        .push(QueuedPanelPrompt::new("next".into(), PromptOrigin::User));
                });
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(45));
    assert!(fixture.activated());
}

#[gpui_kit::test]
fn an_unanswered_brief_keeps_it_awake(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Reset)), cx);
    {
        let mut messages = fixture.messages.lock();
        messages.add(knot_messaging::Message::new(fixture.sender, fixture.agent, "please review"));
        messages.mark_read(fixture.agent);
    }
    let token = fixture.restart_token();
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(45));
    assert_eq!(fixture.restart_token(),
               token,
               "not reset while it owes a reply");
}

#[gpui_kit::test]
fn a_process_started_after_the_session_keeps_it_awake(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Deactivate)), cx);
    let id = fixture.agent;
    fixture.view.update(&mut fixture.window, |view, _| {
                    // The session's first sample is its baseline: an MCP
                    // server.
                    view.note_process_sample(id, 100, &[(101, mins(1))], SystemTime::now());
                    // Later, a test run appears beside it.
                    view.note_process_sample(id,
                                             100,
                                             &[(101, mins(2)), (202, mins(1))],
                                             SystemTime::now());
                });
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(45));
    assert!(fixture.activated(), "the test run is still going");
}

#[gpui_kit::test]
fn the_processes_a_session_starts_with_do_not_keep_it_awake(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Deactivate)), cx);
    let id = fixture.agent;
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.note_process_sample(id, 100, &[(101, mins(1))], SystemTime::now());
                    view.note_process_sample(id, 100, &[(101, mins(1))], SystemTime::now());
                });
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(30));
    assert!(!fixture.activated(),
            "a long-lived MCP server is not activity");
}

#[gpui_kit::test]
fn user_activity_restarts_the_period(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Deactivate)), cx);
    let id = fixture.agent;
    let start = SystemTime::now() - mins(25);
    fixture.check(start);
    // The user scrolls back to read, now - 25 minutes into the period.
    fixture.view
           .update(&mut fixture.window, |view, _| view.note_user_activity(id));
    fixture.check(start + mins(31));
    assert!(fixture.activated(),
            "the period restarted when the user looked");
}

#[gpui_kit::test]
fn an_idle_reset_withholds_the_startup_prompt_and_waits_for_activity(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Reset)), cx);
    let id = fixture.agent;
    let token = fixture.restart_token();
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(30));

    let reset_token = fixture.restart_token();
    assert_ne!(reset_token, token, "reset into a new conversation");
    assert!(fixture.activated(), "a reset keeps the agent running");
    let withheld = fixture.view.update(&mut fixture.window, |view, _| {
                                   view.take_withheld_startup_prompt(id)
                               });
    assert!(withheld,
            "the new session starts without the startup prompt");

    fixture.check(start + mins(120));
    assert_eq!(fixture.restart_token(),
               reset_token,
               "an untouched fresh conversation is not reset again");
}

#[gpui_kit::test]
fn an_agent_that_inherits_follows_the_settings_default(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Inherit, cx);
    fixture.window.update(|_, cx| {
                      crate::settings_global::write(cx, |settings| {
                          settings.idle_default = Some(rule(30, IdleAction::Deactivate));
                      });
                  });
    let start = SystemTime::now();
    fixture.check(start);
    fixture.check(start + mins(30));
    assert!(!fixture.activated(), "the default applied");
}

#[gpui_kit::test]
fn a_reset_agent_that_is_used_can_be_reset_again(cx: &mut TestAppContext) {
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Reset)), cx);
    let id = fixture.agent;
    let start = SystemTime::now() - mins(30);
    fixture.check(start);
    fixture.check(start + mins(30));
    let reset_token = fixture.restart_token();

    // The user uses the fresh conversation, now.
    fixture.view
           .update(&mut fixture.window, |view, _| view.note_user_activity(id));
    fixture.check(start + mins(61));

    assert_ne!(fixture.restart_token(),
               reset_token,
               "used, then idle again: reset again");
}

impl Fixture {
    fn set_state(&mut self, state: knot_agents::AgentState) {
        self.store.lock().set_state(self.agent, state);
    }

    fn sample(&mut self, at: SystemTime, processes: &[(u32, Duration)]) {
        let id = self.agent;
        let processes = processes.to_vec();
        self.view.update(&mut self.window, |view, _| {
                     view.note_process_sample(id, 100, &processes, at);
                 });
    }
}

/// Once the agent has had a turn, a process counts only if that turn (or
/// later) started it: a test run from the turn keeps the agent awake, an
/// older helper does not.
#[gpui_kit::test]
fn a_process_started_in_the_latest_turn_keeps_it_awake(cx: &mut TestAppContext) {
    use knot_agents::AgentState;
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Deactivate)), cx);
    let start = SystemTime::now();
    fixture.sample(start, &[(101, mins(1))]);
    fixture.check(start);
    fixture.set_state(AgentState::Running);
    fixture.check(start + mins(1));
    fixture.set_state(AgentState::Idle);
    // The helper predates the turn; the test run started a minute into it.
    fixture.sample(start + mins(5), &[(101, mins(6)), (202, mins(4))]);
    fixture.check(start + mins(40));
    assert!(fixture.activated(), "the turn's test run is still going");
}

/// A helper started during one turn - a stdio MCP server on first use -
/// stops counting once the next turn begins, so the agent can go idle again.
#[gpui_kit::test]
fn a_helper_from_an_earlier_turn_stops_counting(cx: &mut TestAppContext) {
    use knot_agents::AgentState;
    let mut fixture = fixture(IdlePolicy::Custom(rule(30, IdleAction::Deactivate)), cx);
    let start = SystemTime::now();
    fixture.sample(start, &[]);
    fixture.check(start);
    // Turn one starts the helper.
    fixture.set_state(AgentState::Running);
    fixture.check(start + mins(1));
    fixture.set_state(AgentState::Idle);
    fixture.check(start + mins(3));
    // Turn two.
    fixture.set_state(AgentState::Running);
    fixture.check(start + mins(10));
    fixture.set_state(AgentState::Idle);
    // The helper, started a minute into turn one, is still alive.
    fixture.sample(start + mins(12), &[(303, mins(11))]);
    fixture.check(start + mins(12));
    fixture.check(start + mins(12) + mins(30));
    assert!(!fixture.activated(),
            "a helper from an earlier turn is not activity");
}

//! The scheduled-start ticker against a real store, window registry and
//! settings, with the clock chosen by the test (#404).
//!
//! The agents use an inert agent type, so a started run is observed as the
//! window side records it - the agent activated and restarted - rather than
//! as a launched adapter; `workspace_window::scheduling`'s tests say why.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use jiff::Zoned;
use jiff::civil::date;
use jiff::tz::TimeZone;
use knot_core::StartupPrompt;
use knot_core::schedule::SkipReason;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use super::{ScheduleOutcome, ScheduleOutcomes, tick};
use crate::window_registry::{WindowKey, WindowRegistry};

const INERT_AGENT_TYPE: &str = "schedule-ticker-test-inert";

struct Fixture {
    store:     Arc<Mutex<knot_agents::AgentStore>>,
    messages:  Arc<Mutex<knot_messaging::MessageStore>>,
    workspace: Uuid,
    agent:     Uuid,
    sender:    Uuid,
    _dir:      TempDir,
}

/// A stopped agent scheduled daily at 09:00 with a startup prompt, and a
/// second agent to send it messages. No window is open.
fn fixture(startup_prompt: bool, cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Scheduled");
    let workspace = space.id;
    store.add_workspace(space);
    let options = |prompt: Option<StartupPrompt>| {
        knot_agents::CreateOptions { agent_type: Some(INERT_AGENT_TYPE.into()),
                                     workspace_id: Some(workspace),
                                     startup_prompt: prompt,
                                     ..Default::default() }
    };
    let agent =
        store.create("~/scripted",
                     options(startup_prompt.then(|| {
                                               StartupPrompt::Custom("run the report".into())
                                           })));
    let sender = store.create("~/sender", options(None));
    // Set at 08:00, before the run: a schedule set after its time would wait
    // for the next one (#404).
    store.set_schedule_at(agent,
                          Some("daily 09:00".parse().expect("a preset")),
                          la(8, 0).timestamp());
    store.set_activated(agent, false);
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          cx.set_global(ScheduleOutcomes::default());
      });
    Fixture { store,
              messages,
              workspace,
              agent,
              sender,
              _dir: dir }
}

fn la(hour: i8, minute: i8) -> Zoned {
    date(2026, 11, 16).at(hour, minute, 0, 0)
                      .to_zoned(TimeZone::get("America/Los_Angeles").expect("tzdb has LA"))
                      .expect("a valid local time")
}

impl Fixture {
    fn tick(&self, now: &Zoned, cx: &mut TestAppContext) {
        cx.update(|cx| tick(&self.store, &self.messages, now, cx));
    }

    fn outcome(&self, cx: &mut TestAppContext) -> Option<ScheduleOutcome> {
        cx.update(|cx| cx.global::<ScheduleOutcomes>().0.get(&self.agent).copied())
    }

    fn handled(&self) -> Option<i64> {
        self.store
            .lock()
            .schedule_run_handled(self.workspace, self.agent)
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

    fn window_open(&self, cx: &mut TestAppContext) -> bool {
        cx.update(|cx| {
              WindowRegistry::workspace_view(WindowKey::Workspace(self.workspace), cx).is_some()
          })
    }
}

#[gpui_kit::test]
fn a_due_run_opens_the_window_and_starts_the_agent_once(cx: &mut TestAppContext) {
    let fixture = fixture(true, cx);

    fixture.tick(&la(9, 1), cx);

    assert!(fixture.window_open(cx), "the closed workspace window opens");
    assert!(fixture.activated(), "the stopped agent is started");
    assert_eq!(fixture.handled(), Some(la(9, 0).timestamp().as_second()));
    assert_eq!(fixture.outcome(cx),
               Some(ScheduleOutcome::Started(la(9, 0).timestamp())));
    let persisted = cx.update(|cx| {
                          crate::settings_global::read(cx).workspace_ui
                                                          .get(&fixture.workspace)
                                                          .and_then(|ui| {
                                                              ui.schedule_runs_handled
                                                                .get(&fixture.agent)
                                                          })
                                                          .copied()
                      });
    assert_eq!(persisted,
               fixture.handled(),
               "recorded where a relaunch reads it back");

    let token = fixture.restart_token();
    fixture.tick(&la(9, 2), cx);
    assert_eq!(fixture.restart_token(),
               token,
               "the same run is not started twice");
}

#[gpui_kit::test]
fn an_agent_owing_a_reply_is_skipped_not_started(cx: &mut TestAppContext) {
    let fixture = fixture(true, cx);
    {
        let mut messages = fixture.messages.lock();
        messages.add(knot_messaging::Message::new(fixture.sender, fixture.agent, "review #12"));
        messages.mark_read(fixture.agent);
    }

    fixture.tick(&la(9, 0), cx);

    assert!(!fixture.activated());
    assert_eq!(fixture.outcome(cx),
               Some(ScheduleOutcome::Skipped(la(9, 0).timestamp(), SkipReason::OwesReply)));
    assert!(fixture.handled().is_some(),
            "a skipped run is handled, so it does not start later");
}

#[gpui_kit::test]
fn a_run_past_the_grace_period_is_missed_and_opens_nothing(cx: &mut TestAppContext) {
    let fixture = fixture(true, cx);

    fixture.tick(&la(9, 30), cx);

    assert!(!fixture.window_open(cx));
    assert!(!fixture.activated());
    assert_eq!(fixture.outcome(cx),
               Some(ScheduleOutcome::Missed(la(9, 0).timestamp())));
}

#[gpui_kit::test]
fn an_agent_without_a_startup_prompt_is_never_run(cx: &mut TestAppContext) {
    let fixture = fixture(false, cx);

    fixture.tick(&la(9, 1), cx);

    assert_eq!(fixture.handled(),
               Some(la(8, 0).timestamp().as_second()),
               "only the seed from setting the schedule; the ticker handled nothing");
    assert_eq!(fixture.outcome(cx), None);
    assert!(!fixture.window_open(cx));
}

//! A scheduled run in a real workspace window (#404): what counts as busy,
//! and that a run on a deactivated agent activates it and restarts it once,
//! in a new conversation.
//!
//! The agents use an inert agent type - not a shell, no ACP adapter - so no
//! session actually launches. That pins the window-side sequence (activated,
//! one restart, conversation cleared) but not the adapter spawn count or the
//! startup prompt itself, which need a live session; those are covered by
//! `start_scheduled_run` going through `restart_agents`, the path a manual
//! "Restart with New Conversation" takes.

use std::sync::Arc;

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::{PromptOrigin, QueuedPanelPrompt};

const INERT_AGENT_TYPE: &str = "scheduling-test-inert";

struct Fixture {
    window:   VisualTestContext,
    view:     Entity<WorkspaceWindow>,
    store:    Arc<Mutex<knot_agents::AgentStore>>,
    messages: Arc<Mutex<knot_messaging::MessageStore>>,
    agent:    Uuid,
    other:    Uuid,
    _dir:     TempDir,
}

fn fixture(cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let mut make = |name: &str| {
        let id = store.create(format!("~/{name}"),
                              knot_agents::CreateOptions { agent_type:
                                                               Some(INERT_AGENT_TYPE.into()),
                                                           workspace_id: Some(workspace_id),
                                                           ..Default::default() });
        store.set_acp_session_id(id, format!("acp-{name}"));
        id
    };
    let agent = make("scripted");
    let other = make("sender");
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
              other,
              _dir: dir }
}

impl Fixture {
    fn blocker(&mut self) -> Option<knot_core::schedule::SkipReason> {
        let id = self.agent;
        self.view
            .read_with(&self.window, |view, _| view.scheduled_run_blocker(id))
    }
}

#[gpui_kit::test]
fn an_idle_agent_with_nothing_owed_is_not_busy(cx: &mut TestAppContext) {
    let mut fixture = fixture(cx);
    assert_eq!(fixture.blocker(), None);
}

#[gpui_kit::test]
fn a_queued_prompt_makes_it_busy(cx: &mut TestAppContext) {
    let mut fixture = fixture(cx);
    let id = fixture.agent;
    fixture.view.update(&mut fixture.window, |view, _| {
                    view.panel_prompt_queues
                        .entry(id)
                        .or_default()
                        .push(QueuedPanelPrompt::new("next".into(), PromptOrigin::User));
                });
    assert_eq!(fixture.blocker(),
               Some(knot_core::schedule::SkipReason::PromptQueued));
}

#[gpui_kit::test]
fn an_unanswered_brief_makes_it_busy_until_it_replies(cx: &mut TestAppContext) {
    let mut fixture = fixture(cx);
    let (agent, sender) = (fixture.agent, fixture.other);
    {
        let mut messages = fixture.messages.lock();
        messages.add(knot_messaging::Message::new(sender, agent, "please review #12"));
        messages.mark_read(agent);
    }
    assert_eq!(fixture.blocker(),
               Some(knot_core::schedule::SkipReason::OwesReply),
               "a read, unanswered brief is work in hand");

    fixture.messages
           .lock()
           .add(knot_messaging::Message::new(agent, sender, "done"));
    assert_eq!(fixture.blocker(), None, "a reply settles it");
}

#[gpui_kit::test]
fn a_scheduled_run_activates_a_stopped_agent_and_restarts_it_once(cx: &mut TestAppContext) {
    let mut fixture = fixture(cx);
    let id = fixture.agent;
    fixture.store.lock().set_activated(id, false);
    let token = fixture.store
                       .lock()
                       .agent(id)
                       .expect("exists")
                       .restart_token;

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.start_scheduled_run(id, cx)
                });

    let store = fixture.store.lock();
    let agent = store.agent(id).expect("exists");
    assert!(agent.activated,
            "a stopped agent is started, not only reset");
    assert_ne!(agent.restart_token, token, "it was restarted");
    assert_eq!(agent.acp_session_id, None, "in a new conversation");
}

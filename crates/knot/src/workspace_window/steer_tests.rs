//! Steer now from the window (#26): a queued prompt steered into a running
//! turn, against a fake adapter that answers like `claude-agent-acp` 0.101 -
//! `_meta.steering.supported` on `initialize`, and `{"outcome": ...}` to
//! `_session/steering`. Its first turn never ends, so a turn is running
//! throughout.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{Entity, TestAppContext};
use knot_agent_launch::AdapterConfig;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::panel_session::{self, PanelPhase, PanelSessionSlot};
use crate::panel_state::PanelMessage;
use crate::settings_global;
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::{PromptOrigin, QueuedPanelPrompt};

/// Completes the handshake, advertising steering when `$KNOT_TEST_STEERS`
/// is set, never ends a prompt, and answers a steer with
/// `$KNOT_TEST_OUTCOME`.
fn steering_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
                      id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                      method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
                      meta=""
                      [ -n "$KNOT_TEST_STEERS" ] && meta=",\"_meta\":{\"steering\":{\"supported\":true}}"
                      case "$method" in
                        initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{}$meta}}" ;;
                        session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}" ;;
                        _session/steering) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"outcome\":\"$KNOT_TEST_OUTCOME\"}}" ;;
                      esac
                    done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// A slot whose first turn is running, for an adapter that steers when
/// `steers` and answers steers with `outcome`.
fn busy_slot(runtime: &tokio::runtime::Runtime, steers: bool, outcome: &str)
             -> Arc<Mutex<PanelSessionSlot>> {
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let connecting = Arc::clone(&slot);
    let mut env = vec![("KNOT_TEST_OUTCOME".to_string(), outcome.to_string())];
    if steers {
        env.push(("KNOT_TEST_STEERS".to_string(), "1".to_string()));
    }
    runtime.spawn(async move {
               let config = steering_adapter();
               let request = panel_session::ConnectRequest { config: &config,
                                                             cwd: "/tmp",
                                                             prior_session_id: None,
                                                             mcp_url: None,
                                                             registration_prompt:
                                                                 Some("register".into()),
                                                             resume_registration_prompt: None,
                                                             session_config: BTreeMap::new(),
                                                             default_mode: None,
                                                             option_mode: None,
                                                             session_meta: None,
                                                             env,
                                                             args: Vec::new(),
                                                             subagents: None,
                                                             startup_prompt: None,
                                                             times: None };
               panel_session::connect_into(&connecting, request, &progress, |_| {}).await;
           });
    for _ in 0..500 {
        if slot.lock().phase() == PanelPhase::Ready {
            return slot;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the fake adapter never connected");
}

struct Fixture {
    view:     Entity<WorkspaceWindow>,
    agent:    Uuid,
    slot:     Arc<Mutex<PanelSessionSlot>>,
    entry:    Uuid,
    _dir:     TempDir,
    // Last: the session's tasks run on it, and it must outlive the slot.
    _runtime: tokio::runtime::Runtime,
}

/// A window over one agent mid-turn, with "also the tests" queued behind it.
fn queued_behind_a_turn(steers: bool, outcome: &str, cx: &mut TestAppContext) -> Fixture {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = busy_slot(&runtime, steers, outcome);
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let agent = store.create("/tmp/agent",
                             knot_agents::CreateOptions { workspace_id: Some(workspace_id),
                                                          ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     WindowRegistry::install(cx);
                     settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
                     WorkspaceWindow::open(Arc::clone(&store), messages, workspace_id, cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
                 });
    let queued = QueuedPanelPrompt::new("also the tests".into(), PromptOrigin::User);
    let entry = queued.id;
    cx.update(|cx| {
          view.update(cx, |view, _| {
                  view.panel_sessions.insert(agent, Arc::clone(&slot));
                  view.panel_prompt_queues.insert(agent, vec![queued]);
              });
      });
    Fixture { view,
              agent,
              slot,
              entry,
              _dir: dir,
              _runtime: runtime }
}

impl Fixture {
    /// Steers the queued entry and waits for the agent's answer to settle.
    fn steer(&self, cx: &mut TestAppContext) -> bool {
        let (agent, entry) = (self.agent, self.entry);
        let sent = cx.update(|cx| {
                         self.view
                             .update(cx, |view, _| view.steer_queued_prompt(agent, entry))
                     });
        if sent {
            for _ in 0..500 {
                let settled =
                    cx.update(|cx| self.view.update(cx, |view, _| view.drain_steer_results()));
                if settled {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        sent
    }

    fn queue_texts(&self, cx: &mut TestAppContext) -> Vec<(String, bool, bool)> {
        let agent = self.agent;
        cx.update(|cx| {
              self.view.read(cx).panel_prompt_queues[&agent].iter()
                                                            .map(|p| {
                                                                (p.text.clone(),
                                                                 p.steering,
                                                                 p.failed)
                                                            })
                                                            .collect()
          })
    }

    fn conversation(&self) -> Vec<PanelMessage> {
        match &*self.slot.lock() {
            PanelSessionSlot::Ready(handle) => handle.state().lock().messages.clone(),
            _ => panic!("the session is no longer ready"),
        }
    }
}

#[gpui_kit::test]
fn a_steered_prompt_joins_the_running_turn(cx: &mut TestAppContext) {
    let fixture = queued_behind_a_turn(true, "injected", cx);

    assert!(fixture.steer(cx), "a steer went out");

    assert!(fixture.queue_texts(cx).is_empty(), "it left the queue");
    assert_eq!(fixture.conversation().last(),
               Some(&PanelMessage::User("also the tests".into())),
               "and reads as a prompt in the conversation");
    assert!(match &*fixture.slot.lock() {
                PanelSessionSlot::Ready(handle) => handle.state().lock().turn_active,
                _ => false,
            },
            "the turn it joined is still running");
}

/// The turn ended before the steer landed: the prompt goes next.
#[gpui_kit::test]
fn a_prompt_handed_back_waits_at_the_head(cx: &mut TestAppContext) {
    let fixture = queued_behind_a_turn(true, "promptRequired", cx);

    assert!(fixture.steer(cx));

    assert_eq!(fixture.queue_texts(cx),
               vec![("also the tests".into(), false, false)]);
}

/// An agent that did not advertise steering is offered nothing to steer
/// with, and nothing is sent.
#[gpui_kit::test]
fn an_agent_without_steering_is_not_steered(cx: &mut TestAppContext) {
    let fixture = queued_behind_a_turn(false, "injected", cx);
    let agent = fixture.agent;

    assert!(!cx.update(|cx| fixture.view.read(cx).can_steer(agent)));
    assert!(!fixture.steer(cx));
    assert_eq!(fixture.queue_texts(cx),
               vec![("also the tests".into(), false, false)]);
}

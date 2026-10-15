//! What a landed Autopilot effect does to a Panel-mode agent's conversation
//! (#273): a notice says Autopilot acted, and a reply goes in as a prompt
//! that says where it came from.
//!
//! The session is a real ACP handshake against a fake adapter that never
//! answers `session/prompt`, so the registration turn stays in flight and a
//! reply has to queue behind it - which is where its origin is visible. The
//! settings are rooted at a temporary directory.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{Entity, TestAppContext};
use knot_agent_launch::AdapterConfig;
use knot_mcp_tools::AutopilotEffect;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::panel_session::{self, PanelPhase, PanelSessionSlot};
use crate::panel_state::PanelMessage;
use crate::settings_global;
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

/// Completes the handshake and never answers anything else, so the first
/// turn never ends.
fn stalling_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
                      id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                      method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
                      case "$method" in
                        initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
                        session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}" ;;
                      esac
                    done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// A slot that reaches `Ready` with its registration turn still running.
fn busy_slot(runtime: &tokio::runtime::Runtime) -> Arc<Mutex<PanelSessionSlot>> {
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let connecting = Arc::clone(&slot);
    runtime.spawn(async move {
               let config = stalling_adapter();
               let request = panel_session::ConnectRequest { config:                     &config,
                                                             cwd:                        "/tmp",
                                                             prior_session_id:           None,
                                                             mcp_url:                    None,
                                                             registration_prompt:
                                                                 Some("register".into()),
                                                             resume_registration_prompt: None,
                                                             session_config:
                                                                 BTreeMap::new(),
                                                             default_mode:               None,
                                                             option_mode:                None,
                                                             session_meta:               None,
                                                             env:                        Vec::new(),
                                                             args:                       Vec::new(),
                                                             subagents:                  None,
                                                             startup_prompt:             None, };
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
    _dir:     TempDir,
    // Last: the session's tasks run on it, and it must outlive the slot.
    _runtime: tokio::runtime::Runtime,
}

/// A window over one agent whose panel session is busy with a turn.
fn busy_agent(cx: &mut TestAppContext) -> Fixture {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = busy_slot(&runtime);
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
    cx.update(|cx| {
          view.update(cx, |view, _| {
                  view.panel_sessions.insert(agent, Arc::clone(&slot));
              });
      });
    Fixture { view,
              agent,
              slot,
              _dir: dir,
              _runtime: runtime }
}

impl Fixture {
    fn apply(&self, effect: AutopilotEffect, cx: &mut TestAppContext) {
        let agent = self.agent;
        cx.update(|cx| {
              self.view.update(cx, |view, cx| {
                           view.apply_autopilot_effect(agent, effect, cx)
                       });
          });
    }

    fn conversation(&self) -> Vec<PanelMessage> {
        match &*self.slot.lock() {
            PanelSessionSlot::Ready(handle) => handle.state().lock().messages.clone(),
            _ => panic!("the session is no longer ready"),
        }
    }
}

fn notice(key: &str) -> PanelMessage {
    PanelMessage::Notice(knot_core::l10n::t(key))
}

#[gpui_kit::test]
fn a_mark_says_in_the_conversation_that_autopilot_stopped_here(cx: &mut TestAppContext) {
    let fixture = busy_agent(cx);

    fixture.apply(AutopilotEffect::Mark, cx);

    assert_eq!(fixture.conversation().last(),
               Some(&notice("autopilot.notice.mark")));
}

#[gpui_kit::test]
fn a_reply_is_announced_and_queued_as_autopilot_s(cx: &mut TestAppContext) {
    let fixture = busy_agent(cx);

    fixture.apply(AutopilotEffect::InjectText("yes, continue".into()), cx);

    assert_eq!(fixture.conversation().last(),
               Some(&notice("autopilot.notice.replied")));
    let agent = fixture.agent;
    cx.update(|cx| {
          let view = fixture.view.read(cx);
          let queue = &view.panel_prompt_queues[&agent];
          assert_eq!(queue.len(), 1, "the reply waits behind the running turn");
          assert_eq!(queue[0].text, "yes, continue");
          assert_eq!(queue[0].origin, PromptOrigin::Autopilot);
      });
}

/// An Autopilot reply waiting behind a running turn says so, distinct from
/// every other origin (#273).
#[test]
fn a_queued_autopilot_reply_says_where_it_came_from() {
    use crate::workspace_window::prompt_queue::queued_status_label;

    let label = queued_status_label(false, PromptOrigin::Autopilot);
    assert_eq!(label, knot_core::l10n::t("panel.queued_autopilot"));
    assert_ne!(label, "panel.queued_autopilot", "the key must resolve");
    for other in [PromptOrigin::User,
                  PromptOrigin::InboxNudge,
                  PromptOrigin::Startup]
    {
        assert_ne!(label, queued_status_label(false, other));
    }
}

/// The lines Autopilot adds to a conversation.
#[test]
fn autopilot_notices_resolve() {
    for key in ["autopilot.notice.replied",
                "autopilot.notice.mark",
                "autopilot.notice.ask"]
    {
        assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
    }
}

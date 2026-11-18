//! Holding queued prompts while a session's usage limit stands (#368),
//! through a real `Ready` session in a workspace window: a prompt sent to a
//! limited agent waits, the pump keeps holding it, "send now" lets one past,
//! and the limit lifting lets the rest go.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::panel_session::PanelSessionSlot;
use crate::settings_global;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

/// The issue's error, verbatim.
const LIMIT_ERROR: &str = "The agent could not answer: acp error response (code -32603): \
                           Internal error: You've hit your session limit · resets 5:20pm \
                           (America/Los_Angeles)";

/// The panel state of the `Ready` session in `slot`.
fn state(slot: &Arc<Mutex<PanelSessionSlot>>) -> Arc<Mutex<crate::panel_state::PanelState>> {
    match &*slot.lock() {
        PanelSessionSlot::Ready(handle) => handle.state(),
        _ => panic!("the slot is ready"),
    }
}

#[gpui_kit::test]
fn a_limited_session_holds_its_prompts_until_sent_now_or_the_limit_lifts(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = crate::workspace_window::panel::input::tests::ready_slot(&runtime);

    let mut store = knot_agents::AgentStore::new();
    let space = knot_core::Workspace { id:        Uuid::new_v4(),
                                       name:      "Only".to_string(),
                                       color_hex: "#123456".to_string(),
                                       agent_ids: Vec::new(), };
    let workspace_id = space.id;
    store.add_workspace(space);
    store.set_current_workspace(workspace_id);
    let agent = store.create("~/agent", knot_agents::CreateOptions::default());
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          WorkspaceWindow::open(Arc::clone(&store),
                                Arc::clone(&messages),
                                workspace_id,
                                crate::workspace_window::OpenRoute::Manager,
                                cx);
      });
    let view =
        cx.update(|cx| WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx))
          .expect("opening a workspace registers its view");

    // The last turn ended on the limit.
    state(&slot).lock().push_error(LIMIT_ERROR.to_string());
    assert!(state(&slot).lock().session_limit_lifts_at.is_some(),
            "the issue's reset time is read");

    cx.update(|cx| {
          view.update(cx, |view, _| {
                  view.panel_sessions.insert(agent, Arc::clone(&slot));
                  assert!(view.held_by_session_limit(agent));

                  // Sent while limited: it waits rather than going.
                  assert!(view.deliver_panel_prompt(agent, "first".into(), PromptOrigin::User));
                  assert!(view.deliver_panel_prompt(agent, "second".into(), PromptOrigin::User));
                  assert_eq!(view.panel_prompt_queues[&agent].len(), 2, "both wait");

                  // The pump holds them, tick after tick.
                  assert!(!view.drain_panel_prompt(agent));
                  assert_eq!(view.panel_prompt_queues[&agent].len(), 2);

                  // "Send now" lets one past the limit, ahead of the other.
                  let second = view.panel_prompt_queues[&agent][1].id;
                  assert!(view.send_queued_prompt_now(agent, second));
                  assert!(view.drain_panel_prompt(agent), "the overridden prompt goes");
                  assert_eq!(view.panel_prompts_in_flight[&agent].id, second);
                  assert_eq!(view.panel_prompt_queues[&agent].len(),
                             1,
                             "the other still waits");
              });
      });
}

/// The held state follows the clock: before the reset it holds, after it
/// the queue goes on its own.
#[test]
fn the_hold_lifts_at_the_reset_time() {
    let mut state = crate::panel_state::PanelState::new();
    state.push_error(LIMIT_ERROR.to_string());
    let lifts = state.session_limit_lifts_at
                     .expect("the reset time is read");

    assert!(state.held_by_session_limit(lifts - Duration::from_secs(1)));
    assert!(!state.held_by_session_limit(lifts));
    assert!(!state.held_by_session_limit(lifts + Duration::from_secs(60)));
}

/// A limit whose reset time cannot be read holds until the user sends
/// anyway: a time it is not sure of is no time to send at.
#[test]
fn an_unreadable_reset_holds_until_overridden() {
    let mut state = crate::panel_state::PanelState::new();
    state.push_error("You've hit your usage limit.".to_string());

    assert!(state.session_limit.is_some());
    assert_eq!(state.session_limit_lifts_at, None);
    assert!(state.held_by_session_limit(SystemTime::now() + Duration::from_secs(86_400 * 7)));
}

/// A new prompt is a new try: it clears the limit and its time together.
#[test]
fn a_new_prompt_clears_the_hold() {
    let mut state = crate::panel_state::PanelState::new();
    state.push_error(LIMIT_ERROR.to_string());

    state.push_user_message("try again".to_string());

    assert_eq!(state.session_limit, None);
    assert_eq!(state.session_limit_lifts_at, None);
    assert!(!state.held_by_session_limit(SystemTime::now()));
}

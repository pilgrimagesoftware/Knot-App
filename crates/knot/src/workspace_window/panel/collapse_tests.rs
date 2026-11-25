//! View > Collapse Tool Calls in a real workspace window (#488): the fixed
//! key reaches the selected agent's conversation and collapses it.
//!
//! As in the find bar's tests, the agent's type has no terminal process and
//! no ACP adapter, so nothing launches; its conversation is written straight
//! into the connected fake session's state.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use parking_lot::Mutex;
use tempfile::TempDir;

use crate::panel_session::PanelSessionSlot;
use crate::panel_state::{PanelMessage, PanelState, ToolCallCard};
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;

const INERT_AGENT_TYPE: &str = "collapse-test-inert";

struct Fixture {
    window:    VisualTestContext,
    state:     Arc<Mutex<PanelState>>,
    _settings: TempDir,
    _folder:   TempDir,
    _runtime:  tokio::runtime::Runtime,
}

fn card(id: &str, status: &str) -> PanelMessage {
    PanelMessage::ToolCall(Box::new(ToolCallCard { id:        id.to_owned(),
                                                   kind:      "execute".to_owned(),
                                                   title:     id.to_owned(),
                                                   status:    status.to_owned(),
                                                   content:   Vec::new(),
                                                   raw_input: None,
                                                   meta:      None,
                                                   subagent:  Vec::new(), }))
}

/// A workspace window over one agent whose conversation holds a running and
/// a failed call - both drawn open - with the composer focused.
fn window(cx: &mut TestAppContext) -> Fixture {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = crate::workspace_window::panel::input::tests::ready_slot(&runtime);
    let state = match &*slot.lock() {
        PanelSessionSlot::Ready(handle) => handle.state(),
        _ => panic!("a ready session"),
    };
    state.lock().messages = vec![card("running", "in_progress"), card("failed", "failed")];
    let folder = TempDir::new().expect("a temporary agent folder");
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let agent = store.create(folder.path().to_string_lossy().into_owned(),
                             knot_agents::CreateOptions { agent_type:
                                                              Some(INERT_AGENT_TYPE.into()),
                                                          workspace_id: Some(workspace_id),
                                                          ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let settings_dir = TempDir::new().expect("a temporary settings root");
    let settings = knot_core::Settings::with_store_root(settings_dir.path());
    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     cx.bind_keys(crate::keymap::fixed_bindings().into_iter()
                                                                 .map(|fixed| fixed.binding));
                     WindowRegistry::install(cx);
                     crate::settings_global::install(settings, cx);
                     WorkspaceWindow::open(store,
                                           messages,
                                           workspace_id,
                                           crate::workspace_window::OpenRoute::Manager,
                                           cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
                 });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    let mut window = VisualTestContext::from_window(handle, cx);
    let composer = view.update_in(&mut window, |view, window, cx| {
                           view.panel_sessions.insert(agent, slot);
                           view.store.lock().set_activated(agent, true);
                           view.selected_agent = Some(agent);
                           cx.notify();
                           view.panel_prompt_input(agent, window, cx)
                       });
    window.run_until_parked();
    composer.update_in(&mut window, |state, window, cx| state.focus(window, cx));
    window.run_until_parked();
    Fixture { window,
              state,
              _settings: settings_dir,
              _folder: folder,
              _runtime: runtime }
}

impl Fixture {
    fn open_cards(&self) -> Vec<String> {
        let state = self.state.lock();
        state.messages
             .iter()
             .filter_map(|message| match message {
                 PanelMessage::ToolCall(card) if !state.is_collapsed(card) => Some(card.id.clone()),
                 _ => None,
             })
             .collect()
    }
}

#[gpui_kit::test]
fn the_collapse_key_collapses_the_selected_conversations_tool_calls(cx: &mut TestAppContext) {
    let mut fixture = window(cx);
    assert_eq!(fixture.open_cards(), ["running", "failed"]);

    fixture.window.simulate_keystrokes("secondary-alt-c");
    fixture.window.run_until_parked();

    assert!(fixture.open_cards().is_empty(), "every card is collapsed");
}

#[gpui_kit::test]
fn the_collapse_menu_item_is_enabled_with_a_conversation_on_screen(cx: &mut TestAppContext) {
    let mut fixture = window(cx);
    let available =
        fixture.window.update(|window, cx| {
                          window.is_action_available(&crate::app_bootstrap::CollapseToolCalls, cx)
                      });
    assert!(available);
}

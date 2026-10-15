//! Moving an agent between two open workspace windows (#247): the
//! destination is told, the source lets go of the agent's session, and the
//! conversation survives the hand-over.
//!
//! The agent uses an agent type with no terminal process and no ACP
//! adapter, so nothing launches; what is asserted is what each window is
//! told and what the store is left holding.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use super::agent_row::AgentMenuTargets;
use super::agent_row::move_agent_to_workspace;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "move-test-inert";

struct Fixture {
    store:       Arc<Mutex<knot_agents::AgentStore>>,
    source:      Entity<WorkspaceWindow>,
    destination: Entity<WorkspaceWindow>,
    source_id:   Uuid,
    target_id:   Uuid,
    agent:       Uuid,
    _dir:        TempDir,
}

/// Two open workspace windows, `Source` holding one agent with a
/// conversation and `Target` empty.
fn two_open_workspaces(cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let source = crate::tests::workspace("Source");
    let target = crate::tests::workspace("Target");
    let (source_id, target_id) = (source.id, target.id);
    store.add_workspace(source);
    store.add_workspace(target);
    let agent = store.create("~/agent",
                             knot_agents::CreateOptions { agent_type:
                                                              Some(INERT_AGENT_TYPE.into()),
                                                          workspace_id: Some(source_id),
                                                          ..Default::default() });
    store.set_session_id(agent, "session-0".to_string());
    store.set_acp_session_id(agent, "acp-0".to_string());
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let (source, destination) =
        cx.update(|cx| {
              gpui_kit::init(cx);
              WindowRegistry::install(cx);
              crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
              let mut open = |id| {
                  WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), id, cx);
                  WindowRegistry::workspace_view(WindowKey::Workspace(id), cx)
                .expect("opening a workspace registers its view")
              };
              (open(source_id), open(target_id))
          });
    Fixture { store,
              source,
              destination,
              source_id,
              target_id,
              agent,
              _dir: dir }
}

impl Fixture {
    fn move_to(&self, workspace_id: Uuid, cx: &mut TestAppContext) {
        let targets = AgentMenuTargets { store:         Arc::clone(&self.store),
                                         window_entity: self.source.clone(),
                                         workspace_id:  self.source_id,
                                         id:            self.agent,
                                         name:          "agent".to_string(),
                                         folder:        "~/agent".to_string(), };
        cx.update(|cx| move_agent_to_workspace(&targets, workspace_id, cx));
    }

    /// Counts the notifications `view` receives from here on.
    fn notifications(view: &Entity<WorkspaceWindow>, cx: &mut TestAppContext) -> Rc<Cell<usize>> {
        let count = Rc::new(Cell::new(0));
        let counter = Rc::clone(&count);
        cx.update(|cx| {
              cx.observe(view, move |_, _| counter.set(counter.get() + 1))
                .detach();
          });
        count
    }

    fn restart_token(&self) -> Uuid {
        self.store
            .lock()
            .agent(self.agent)
            .expect("the agent exists")
            .restart_token
    }
}

#[gpui_kit::test]
fn an_open_destination_is_told_the_agent_arrived(cx: &mut TestAppContext) {
    let fixture = two_open_workspaces(cx);
    // Whatever opening the windows scheduled lands first, so the count
    // below is the move's alone.
    cx.run_until_parked();
    let notified = Fixture::notifications(&fixture.destination, cx);

    fixture.move_to(fixture.target_id, cx);
    cx.run_until_parked();

    assert!(notified.get() > 0,
            "the destination window was never notified");
    let members =
        crate::workspace_window::workspace_agent_ids(&fixture.store.lock(), fixture.target_id);
    assert_eq!(members, vec![fixture.agent]);
}

#[gpui_kit::test]
fn the_source_lets_go_of_the_session_and_keeps_the_conversation(cx: &mut TestAppContext) {
    let fixture = two_open_workspaces(cx);
    let token = fixture.restart_token();

    fixture.move_to(fixture.target_id, cx);

    assert_ne!(fixture.restart_token(),
               token,
               "the source's session is torn down");
    let agent = fixture.store
                       .lock()
                       .agent(fixture.agent)
                       .cloned()
                       .expect("the agent exists");
    assert_eq!(agent.acp_session_id.as_deref(), Some("acp-0"));
    assert_eq!(agent.resume_session_id.as_deref(), Some("session-0"));
    let selected = cx.update(|cx| fixture.source.read(cx).selected_agent);
    assert_ne!(selected, Some(fixture.agent));
}

#[gpui_kit::test]
fn moving_into_the_workspace_it_is_in_restarts_nothing(cx: &mut TestAppContext) {
    let fixture = two_open_workspaces(cx);
    let token = fixture.restart_token();

    fixture.move_to(fixture.source_id, cx);

    assert_eq!(fixture.restart_token(), token);
}

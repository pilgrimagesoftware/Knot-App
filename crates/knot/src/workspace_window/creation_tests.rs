//! Showing a newly created agent (#269): it is selected either way, and
//! started only when its activation mode says so. A duplicate is a created
//! agent too, with its source's mode (#289).
//!
//! The agents use an agent type with no terminal process and no ACP
//! adapter, so nothing launches; `activated` is what decides whether the
//! window would start one.

use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use knot_core::ActivationMode;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "creation-test-inert";

struct Fixture {
    view:         Entity<WorkspaceWindow>,
    store:        Arc<Mutex<knot_agents::AgentStore>>,
    workspace_id: Uuid,
    _dir:         TempDir,
}

/// An open window over an empty workspace.
fn open_workspace(cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let view = cx.update(|cx| {
                   gpui_kit::init(cx);
                   WindowRegistry::install(cx);
                   crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()),
                                                     cx);
                   WorkspaceWindow::open(Arc::clone(&store), messages, workspace_id, cx);
                   WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
               });
    Fixture { view,
              store,
              workspace_id,
              _dir: dir }
}

impl Fixture {
    /// Creates an agent the way the editor does, then shows it the way the
    /// editor's callback does.
    fn create_and_show(&self, activation_mode: ActivationMode, cx: &mut TestAppContext) -> Uuid {
        let id = self.store
                     .lock()
                     .create("~/agent",
                             knot_agents::CreateOptions { agent_type: Some(INERT_AGENT_TYPE.into()),
                                                          workspace_id: Some(self.workspace_id),
                                                          activation_mode,
                                                          ..Default::default() });
        cx.update(|cx| {
              self.view
                  .update(cx, |view, cx| view.show_created_agent(id, cx));
          });
        id
    }

    /// Duplicates `source` the way the agent menu's Duplicate does.
    fn duplicate(&self, source: Uuid, cx: &mut TestAppContext) -> Uuid {
        cx.update(|cx| {
              self.view
                  .update(cx, |view, cx| view.duplicate_agent(source, cx))
          })
          .expect("the source exists")
    }

    fn activated(&self, id: Uuid) -> bool {
        self.store
            .lock()
            .agent(id)
            .expect("the agent exists")
            .activated
    }

    fn selected(&self, cx: &mut TestAppContext) -> Option<Uuid> {
        cx.update(|cx| self.view.read(cx).selected_agent)
    }
}

#[gpui_kit::test]
fn a_new_passive_agent_is_shown_but_not_started(cx: &mut TestAppContext) {
    let fixture = open_workspace(cx);

    let id = fixture.create_and_show(ActivationMode::Passive, cx);

    assert_eq!(fixture.selected(cx), Some(id));
    assert!(!fixture.activated(id),
            "a passive agent started on creation");
}

#[gpui_kit::test]
fn a_new_active_agent_is_shown_and_started(cx: &mut TestAppContext) {
    let fixture = open_workspace(cx);

    let id = fixture.create_and_show(ActivationMode::Active, cx);

    assert_eq!(fixture.selected(cx), Some(id));
    assert!(fixture.activated(id));
}

/// The reported defect: Duplicate selected the copy as if the user had
/// chosen it, which activated - and so started - a passive agent. The source
/// is running here, the case that made it easy to miss: the copy must
/// follow the source's mode, not its run state.
#[gpui_kit::test]
fn duplicating_a_passive_agent_does_not_start_the_copy(cx: &mut TestAppContext) {
    let fixture = open_workspace(cx);
    let source = fixture.create_and_show(ActivationMode::Passive, cx);
    fixture.store.lock().set_activated(source, true);

    let copy = fixture.duplicate(source, cx);

    assert_ne!(copy, source);
    assert_eq!(fixture.selected(cx), Some(copy), "the copy is shown");
    assert!(!fixture.activated(copy),
            "a duplicate of a passive agent started");
    assert!(fixture.activated(source),
            "duplicating left the source as it was");
}

#[gpui_kit::test]
fn duplicating_an_active_agent_starts_the_copy(cx: &mut TestAppContext) {
    let fixture = open_workspace(cx);
    let source = fixture.create_and_show(ActivationMode::Active, cx);

    let copy = fixture.duplicate(source, cx);

    assert_eq!(fixture.selected(cx), Some(copy));
    assert!(fixture.activated(copy));
}

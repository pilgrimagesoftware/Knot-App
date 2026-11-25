//! A workspace window over a named roster, laid out as a test asks, for the
//! tests of `workspace-layouts`' window behaviour (`layout_select_tests`,
//! `layout_panes_tests`).
//!
//! Agents are identified by name, in sidebar order, so an assertion reads
//! like its scenario. They use an agent type with no terminal process and no
//! ACP adapter, so starting one launches nothing, and they are Panel-mode, so
//! each pane's input is its composer. Settings are rooted at a temporary
//! directory: opening and selecting persist the UI-state document.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::Focusable;
use gpui_kit::Modifiers;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::px;
use gpui_kit::size;
use knot_core::LayoutMode;
use knot_core::WorkspaceUiState;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "layout-test-inert";

/// The window's size, wide enough that the sidebar is not compact.
const WINDOW: (f32, f32) = (1200., 800.);

/// What a window opens with.
pub(super) struct Spec<'a> {
    /// The roster in sidebar order, before companions are placed.
    pub(super) agents:     &'a [&'a str],
    /// `(owner, companion)`: each companion's row follows its owner's.
    pub(super) companions: &'a [(&'a str, &'a str)],
    pub(super) mode:       LayoutMode,
    /// The agents in the panes, in order.
    pub(super) panes:      &'a [&'a str],
    pub(super) focus:      i32,
}

pub(super) struct Fixture {
    pub(super) window:    VisualTestContext,
    pub(super) view:      Entity<WorkspaceWindow>,
    pub(super) store:     Arc<Mutex<knot_agents::AgentStore>>,
    pub(super) workspace: Uuid,
    pub(super) document:  PathBuf,
    ids:                  BTreeMap<String, Uuid>,
}

pub(super) fn open(spec: &Spec, cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Layouts");
    let workspace = space.id;
    store.add_workspace(space);
    let mut ids = BTreeMap::new();
    let create =
        |store: &mut knot_agents::AgentStore, name: &str, options: knot_agents::CreateOptions| {
            store.create(format!("~/{name}"),
                         knot_agents::CreateOptions { name: Some(name.into()),
                                                      agent_type: Some(INERT_AGENT_TYPE.into()),
                                                      workspace_id: Some(workspace),
                                                      ..options })
        };
    for name in spec.agents {
        ids.insert((*name).to_string(),
                   create(&mut store, name, knot_agents::CreateOptions::default()));
    }
    for (owner, name) in spec.companions {
        let owner = ids[*owner];
        let id = create(&mut store,
                        name,
                        knot_agents::CreateOptions { is_companion: true,
                                                     created_by: Some(owner),
                                                     insert_after: Some(owner),
                                                     ..Default::default() });
        ids.insert((*name).to_string(), id);
    }
    store.set_workspace_layout(workspace,
                               spec.mode,
                               spec.panes.iter().map(|name| ids[*name]).collect());
    store.set_workspace_focused_pane(workspace, spec.focus);

    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = tempfile::tempdir().expect("a temporary settings root");
    let document = dir.path().join(knot_core::consts::WORKSPACE_UI_STATE_FILE);
    let settings = knot_core::Settings::with_store_root(dir.path());
    // The directory has to outlive the test; the OS reaps it.
    std::mem::forget(dir);
    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     WindowRegistry::install(cx);
                     cx.set_global(crate::menu_bar::MenuBarState::default());
                     crate::settings_global::install(settings, cx);
                     crate::keymap::register_global_handlers(Arc::clone(&store), cx);
                     WorkspaceWindow::open(Arc::clone(&store),
                                           messages,
                                           workspace,
                                           crate::workspace_window::OpenRoute::Manager,
                                           cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace), cx)
                         .expect("opening a workspace registers its view")
                 });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    let window = VisualTestContext::from_window(handle, cx);
    window.simulate_resize(size(px(WINDOW.0), px(WINDOW.1)));
    window.run_until_parked();
    Fixture { window,
              view,
              store,
              workspace,
              document,
              ids }
}

impl Fixture {
    pub(super) fn id(&self, name: &str) -> Uuid {
        self.ids[name]
    }

    /// The name of agent `id`.
    pub(super) fn name(&self, id: Uuid) -> String {
        self.ids
            .iter()
            .find(|(_, candidate)| **candidate == id)
            .map(|(name, _)| name.clone())
            .expect("an agent of this fixture")
    }

    pub(super) fn ui(&self) -> WorkspaceUiState {
        self.store.lock().workspace_ui(self.workspace)
    }

    /// The agents in the panes, by name, in pane order.
    pub(super) fn panes(&self) -> Vec<String> {
        self.ui()
            .active_agent_ids
            .into_iter()
            .map(|id| self.name(id))
            .collect()
    }

    pub(super) fn selected(&mut self) -> Option<String> {
        let id = self.view
                     .read_with(&self.window, |view, _| view.selected_agent)?;
        Some(self.name(id))
    }

    pub(super) fn is_activated(&self, name: &str) -> bool {
        self.store
            .lock()
            .agent(self.id(name))
            .is_some_and(|agent| agent.activated)
    }

    /// The row `name` has in the sidebar, counting companions' rows.
    pub(super) fn row(&self, name: &str) -> usize {
        let order = {
            let store = self.store.lock();
            super::workspace_agent_ids(&store, self.workspace)
        };
        order.iter()
             .position(|id| *id == self.id(name))
             .expect("a row for the agent")
    }

    /// Whether `name`'s sidebar row is the highlighted one.
    pub(super) fn row_highlighted(&mut self, name: &str) -> bool {
        let selector: &'static str =
            Box::leak(format!("workspace-agent-selected-{}", self.row(name)).into_boxed_str());
        self.window.debug_bounds(selector).is_some()
    }

    /// Clicks `name`'s sidebar row with `modifiers` held.
    pub(super) fn click_row(&mut self, name: &str, modifiers: Modifiers) {
        let selector: &'static str =
            Box::leak(format!("workspace-agent-row-{}", self.row(name)).into_boxed_str());
        let at = self.window
                     .debug_bounds(selector)
                     .unwrap_or_else(|| panic!("{selector} was never painted"))
                     .center();
        self.window.simulate_click(at, modifiers);
        self.window.run_until_parked();
    }

    /// Whether `name`'s composer holds keyboard focus.
    pub(super) fn composer_focused(&mut self, name: &str) -> bool {
        let id = self.id(name);
        self.view.update_in(&mut self.window, |view, window, cx| {
                     view.panel_prompt_input(id, window, cx)
                         .focus_handle(cx)
                         .is_focused(window)
                 })
    }

    pub(super) fn press(&mut self, action: impl gpui_kit::Action) {
        self.window.dispatch_action(action);
        self.window.run_until_parked();
    }
}

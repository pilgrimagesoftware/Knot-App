//! Opening a workspace window: the GPUI window itself, the initial
//! selection, the subscriptions that keep it in step with the store, and
//! the repaint poll that drives everything that changes off the main
//! thread.
//!
//! The poll is the reason this is long. GPUI redraws on notification, and
//! a session's output, a spinner frame and a panel phase change all arrive
//! from other threads, so one timer asks [`super::repaint`]'s predicates
//! what has moved and notifies when something has.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Entity;
use gpui_kit::component::Root;
use gpui_kit::component::resizable::ResizableState;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::app_state::agent_selection_for_workspace;
use crate::app_support::observe_system_appearance;
use crate::dashboard;
use crate::window_options::reconciled_workspace_bounds;
use crate::window_options::workspace_window_options;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::OpenRoute;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::repaint::spawn_repaint_poll;
use crate::workspace_window::terminal_font::TerminalFont;
use crate::workspace_window::workspace_title;

/// One request to open or raise a workspace window.
struct OpenRequest {
    workspace_id: Uuid,
    select_agent: Option<Uuid>,
    /// False opens a new window without making it key or activating the
    /// app, and leaves an open one where it is.
    take_focus:   bool,
    route:        OpenRoute,
}

impl WorkspaceWindow {
    pub(crate) fn open(store: Arc<Mutex<knot_agents::AgentStore>>,
                       messages: Arc<Mutex<knot_messaging::MessageStore>>, workspace_id: Uuid,
                       route: OpenRoute, cx: &mut App) {
        Self::open_with_selection(store, messages, workspace_id, None, route, cx);
    }

    /// Opens the window of the workspace that holds agent `id`, with `id`
    /// selected and shown - or raises that window and shows `id` in it, if
    /// it is already open. Reports whether `id` belongs to a workspace at
    /// all: an agent removed since, or an id that was never an agent's,
    /// opens nothing.
    ///
    /// Every route that knows only an agent - a Command Center card, a
    /// clicked notification (#1) - comes here, so they cannot disagree about
    /// which window an agent lives in.
    pub(crate) fn open_agent(store: Arc<Mutex<knot_agents::AgentStore>>,
                             messages: Arc<Mutex<knot_messaging::MessageStore>>, id: Uuid,
                             route: OpenRoute, cx: &mut App)
                             -> bool {
        let workspace_id = store.lock()
                                .workspaces()
                                .iter()
                                .find(|workspace| workspace.agent_ids.contains(&id))
                                .map(|workspace| workspace.id);
        let Some(workspace_id) = workspace_id
        else {
            return false;
        };
        Self::open_with_selection(store, messages, workspace_id, Some(id), route, cx);
        true
    }

    /// Like `open`, but overrides the agent that would otherwise be picked
    /// by `agent_selection_for_workspace` - used when a caller (e.g. a
    /// Command Center card) already knows which agent the user wants to
    /// land on.
    pub(crate) fn open_with_selection(store: Arc<Mutex<knot_agents::AgentStore>>,
                                      messages: Arc<Mutex<knot_messaging::MessageStore>>,
                                      workspace_id: Uuid, select_agent: Option<Uuid>,
                                      route: OpenRoute, cx: &mut App) {
        Self::open_window(store,
                          messages,
                          OpenRequest { workspace_id,
                                        select_agent,
                                        take_focus: true,
                                        route },
                          cx);
    }

    /// The workspace window for a scheduled run (#404): the open one as it
    /// is, without raising it, or a new one that takes focus only when Knot
    /// is already the active app - a run at 09:00 must not pull the user out
    /// of whatever other app they are in. `None` if no window could be made.
    pub(crate) fn open_for_schedule(store: Arc<Mutex<knot_agents::AgentStore>>,
                                    messages: Arc<Mutex<knot_messaging::MessageStore>>,
                                    workspace_id: Uuid, cx: &mut App)
                                    -> Option<Entity<Self>> {
        let key = WindowKey::Workspace(workspace_id);
        if let Some(view) = WindowRegistry::workspace_view(key, cx) {
            return Some(view);
        }
        let knot_is_active = cx.active_window().is_some();
        Self::open_window(store,
                          messages,
                          OpenRequest { workspace_id,
                                        select_agent: None,
                                        take_focus: knot_is_active,
                                        route: OpenRoute::ScheduledStart },
                          cx);
        WindowRegistry::workspace_view(key, cx)
    }

    /// Opens or raises `request`'s window, and logs which it did and why.
    fn open_window(store: Arc<Mutex<knot_agents::AgentStore>>,
                   messages: Arc<Mutex<knot_messaging::MessageStore>>, request: OpenRequest,
                   cx: &mut App) {
        let OpenRequest { workspace_id,
                          select_agent,
                          take_focus,
                          route, } = request;
        // One window per workspace: a second request raises the first rather
        // than opening another, and shows the agent it named if it named one
        // (`openspec/specs/window-lifecycle`). Every route that opens a
        // workspace - the manager, a Command Center card or heading, the agent
        // editor's post-create jump - arrives here, so the check belongs here
        // rather than at each of them.
        let key = WindowKey::Workspace(workspace_id);
        if !take_focus && WindowRegistry::workspace_view(key, cx).is_some() {
            return;
        }
        if WindowRegistry::activate(key, cx) {
            eprintln!("window raised: workspace {workspace_id} via {route}");
            if let Some(requested) = select_agent
               && let Some(view) = WindowRegistry::workspace_view(key, cx)
            {
                view.update(cx, |view, cx| view.reveal_agent(requested, cx));
            }
            return;
        }
        // Through the same resolver the title bar renders from, so the OS
        // title and the drawn one agree by construction rather than by two
        // lookups that happen to match. The fallback is only reachable for a
        // workspace that is already gone, whose window draws
        // `workspace.missing` instead.
        let workspace_name =
            workspace_title(&store.lock(), workspace_id).unwrap_or_else(|| "Workspace".to_string());
        let saved_bounds = store.lock().workspace_ui(workspace_id).window_bounds;
        let placed = reconciled_workspace_bounds(saved_bounds, cx);
        let mut options = workspace_window_options(placed, cx);
        options.focus = take_focus;
        if let Err(error) =
            cx.open_window(options, move |window, cx| {
                  // Every window tracks the OS appearance, so a light/dark flip
                  // re-resolves the system palette and repaints.
                  observe_system_appearance(window);
                  // The OS window title (Mission Control, Cmd+`, Window menu)
                  // is separate from the TitleBar row we draw
                  // ourselves - without this it falls back to
                  // the app's bundle name for every workspace
                  // window.
                  window.set_window_title(&workspace_name);
                  let window_id = window.window_handle().window_id();
                  eprintln!("window opened: workspace {workspace_id} window {} via {route}",
                            window_id.as_u64());
                  // The view keeps the subscription, so the hook goes when
                  // the view does and fires for this window alone.
                  let window_closed =
                      cx.on_window_closed(move |_, closed| {
                            if closed == window_id {
                                eprintln!("window closed: workspace {workspace_id} window {}",
                                          window_id.as_u64());
                            }
                        });
                  // Order the new window front rather than letting it open
                  // behind whatever has focus - matching what the settings
                  // window already does when it reuses an open one.
                  if take_focus {
                      window.activate_window();
                  }
                  let selected_agent = select_agent.or_else(|| {
                                                       let store = store.lock();
                                                       agent_selection_for_workspace(&store,
                                                                                     workspace_id)
                                                   });
                  // Before the entity closure: the accessor needs `&mut App`,
                  // which `cx.new`'s context is not.
                  let subagents = crate::app_support::Subagents::handle(cx);
                  let sidebar_resize = cx.new(|_| ResizableState::default());
                  let exited_sessions: Arc<Mutex<Vec<Uuid>>> = Arc::new(Mutex::new(Vec::new()));
                  let view =
                      cx.new(|cx| {
                            let mut window = WorkspaceWindow {
                    exited_sessions: Arc::clone(&exited_sessions),
                    window_bounds_subscription: None,
                    _window_closed: Some(window_closed),
                    diff_stats: crate::diff_stats::DiffStatsCache::default(),
                    git: super::git_panel_state::GitPanelState::default(),
                    artifact_panel: BTreeMap::new(),
                    artifact_panel_resize: BTreeMap::new(),
                    artifact_split_resize: BTreeMap::new(),
                    artifact_drawn: BTreeMap::new(),
                    artifact_closed: BTreeMap::new(),
                    checklist_inputs: BTreeMap::new(),
                    markdown_documents: BTreeMap::new(),
                    pull_request_states:
                        crate::pull_request_state::PullRequestStateCache::default(),
                    forge_status: crate::pull_request_state::ForgeStatus::default(),
                    pull_request_open_failed: false,
                    pull_request_view: Default::default(),
                    changes_view: Default::default(),
                    work_items: Default::default(),
                    codex_usage: Default::default(),
                    codex_usage_dismissed: None,
                    codex_usage_source: crate::codex_usage::read_current,
                    open_config_selector: None,
                    prompt_picker: None,
                    bench_popover_open: false,
                    bench_popover_hovered: None,
                    store,
                    messages,
                    nudged_messages: BTreeMap::new(),
                    panel_handoffs: BTreeMap::new(),
                    drawn_store_revision: 0,
                    start_requested: std::collections::BTreeSet::new(),
                    notified_awaiting: BTreeMap::new(),
                    workspace_id,
                    divider_drag: None,
                    frame_layout: None,
                    selected_agent,
                    sessions: BTreeMap::new(),
                    terminal_panes: BTreeMap::new(),
                    panel_states: BTreeMap::new(),
                    runtime: crate::window_runtime::WindowRuntime::new()
                        .expect("failed to start terminal session runtime"),
                    sidebar_resize,
                    root_focus: cx.focus_handle(),
                    key_hints: Default::default(),
                    shift_held: false,
                    terminal_font: TerminalFont::default(),
                    panel_sessions: BTreeMap::new(),
                    subagents: Arc::clone(&subagents),
                    last_spinner_frame: 0,
                    focused_pane: None,
                    panel_phases: BTreeMap::new(),
                    panel_prompt_inputs: BTreeMap::new(),
                    prompt_inputs_sheets: BTreeMap::new(),
                    orphaned_prompt_inputs_sheets: Vec::new(),
                    panel_selectors: BTreeMap::new(),
                    panel_selector_subscriptions: BTreeMap::new(),
                    panel_selector_items: BTreeMap::new(),
                    panel_prompt_input_subscriptions: BTreeMap::new(),
                    panel_prompt_queues: BTreeMap::new(),
                    agent_idle: BTreeMap::new(),
                    idle_checked_at: None,
                    panel_prompts_in_flight: BTreeMap::new(),
                    panel_trackers: BTreeMap::new(),
                    panel_reported_states: BTreeMap::new(),
                    panel_status_landed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    panel_stopping: BTreeSet::new(),
                    panel_prompt_results: Arc::new(Mutex::new(Vec::new())),
                    panel_steer_results: Arc::new(Mutex::new(Vec::new())),
                    panel_lists: BTreeMap::new(),
                    panel_list_row_counts: BTreeMap::new(),
                    panel_finds: BTreeMap::new(),
                    panel_questions: BTreeMap::new(),
                    window_handle: window.window_handle(),
                    titled_as: workspace_name.clone(),
                    panel_pending_context: BTreeMap::new(),
                    panel_composer_styling: BTreeMap::new(),
                    panel_mentions: BTreeMap::new(),
                    dashboard_quick_prompts: Default::default(),
                    file_finder: None,
                    panel_pending_attachments: BTreeMap::new(),
                    panel_shell_runs: Arc::new(Mutex::new(BTreeMap::new())),
                    panel_input_expanded: BTreeSet::new(),
                    panel_lookups: BTreeMap::new(),
                    process_sections: BTreeMap::new(),
                    process_publish: Arc::new(Mutex::new(
                        crate::agent_processes::Published::default(),
                    )),
                    process_generation: 0,
                    process_sampled_at: None,
                    process_sampling: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    process_failures: Arc::new(Mutex::new(Vec::new())),
                    mcp_sections: BTreeMap::new(),
                    mcp_in_flight: Arc::new(Mutex::new(BTreeSet::new())),
                    mcp_results: Arc::new(Mutex::new(Vec::new())),
                    mcp_tools_in_flight: Arc::new(Mutex::new(BTreeSet::new())),
                    mcp_tool_results: Arc::new(Mutex::new(Vec::new())),
                    view_mode: WorkspaceViewMode::Terminal,
                    dashboard_sort: dashboard::DashboardSort::default(),
                    error: None,
                    agent_drag: crate::workspace_window::AgentRowDrag::default(),
                };
                            // Before anything starts: a handover terminal
                            // left by an earlier window would otherwise
                            // re-run its MCP flow unasked (#443).
                            window.remove_stale_mcp_handovers(cx);
                            // Matches the Swift reference: every agent in the
                            // workspace starts its session when the workspace
                            // window opens, not
                            // just the one initially selected.
                            let agent_ids: Vec<Uuid> =
                                {
                                    let store = window.store.lock();
                                    store.workspaces()
                                         .iter()
                                         .find(|workspace| workspace.id == workspace_id)
                                         .map(|workspace| workspace.agent_ids.clone())
                                }.unwrap_or_default();
                            let mut requested_layout = None;
                            {
                                let mut store = window.store.lock();
                                // The workspace's `active` agents start
                                // here, and only they: `activated` is
                                // runtime-only and loads false, so the
                                // durable mode is consulted on every open.
                                store.activate_on_workspace_open(&agent_ids);
                                // An explicitly requested agent (a Command
                                // Center card) is a selection, and starts
                                // whatever its mode. The workspace's own
                                // restored selection is not, so a `passive`
                                // agent stays stopped across a relaunch.
                                //
                                // It goes through the layout's selection rules
                                // like a sidebar click, so a restored split
                                // that does not show it gains it in a pane,
                                // and what that placed starts too.
                                if let Some(requested) = select_agent {
                                    store.set_activated(requested, true);
                                    let change =
                                        store.select_in_layout(workspace_id, requested, false);
                                    for placed in change.placed() {
                                        store.set_activated(placed, true);
                                    }
                                    requested_layout = Some(change);
                                }
                            }
                            // Written after the guard is released, the same
                            // blocking-I/O rule `persist_workspace_ui` keeps.
                            if requested_layout.is_some_and(|change| change.changed()) {
                                window.persist_workspace_ui(cx);
                            }
                            for id in agent_ids {
                                window.ensure_session(id, cx);
                                window.ensure_panel_session(id, cx);
                            }
                            window
                        });
                  // Registered from inside the open closure because the
                  // view is only in scope here: `cx.open_window` hands back a
                  // handle to the `Root` wrapper, not to this.
                  WindowRegistry::register(key, window.window_handle(), Some(view.downgrade()), cx);
                  spawn_repaint_poll(view.downgrade(), Arc::clone(&exited_sessions), cx);
                  // Remember where the user puts this workspace's window.
                  // The observer fires continuously through a drag, so the
                  // store's setter reports whether the frame actually
                  // changed and only then is anything written to disk.
                  view.update(cx, |view, cx| {
                          let subscription =
                              cx.observe_window_bounds(window, move |view, window, cx| {
                                    let bounds = window.window_bounds().get_bounds();
                                    // Our own placement is not a move the user
                                    // made. Writing it back would replace the
                                    // remembered frame with the one we fell
                                    // back to, so a window arranged on a
                                    // display that is merely unplugged would
                                    // lose its place the first time it was
                                    // reopened without it.
                                    if Some(bounds) == placed {
                                        return;
                                    }
                                    let saved =
                                        knot_core::SavedWindowBounds { x:      bounds.origin
                                                                                     .x
                                                                                     .into(),
                                                                       y:      bounds.origin
                                                                                     .y
                                                                                     .into(),
                                                                       width:  bounds.size
                                                                                     .width
                                                                                     .into(),
                                                                       height: bounds.size
                                                                                     .height
                                                                                     .into(), };
                                    let changed =
                                        view.store
                                            .lock()
                                            .set_workspace_window_bounds(workspace_id, saved);
                                    if changed {
                                        // The UI-state document alone: a
                                        // drag must not rewrite the roster.
                                        view.persist_workspace_ui(cx);
                                    }
                                });
                          view.window_bounds_subscription = Some(subscription);
                      });
                  cx.new(|cx| Root::new(view, window, cx))
              })
        {
            eprintln!("failed to open workspace window: {error}");
        }
    }
}

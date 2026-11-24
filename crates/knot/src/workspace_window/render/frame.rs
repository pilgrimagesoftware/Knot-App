//! What a frame does before it draws: the store snapshot it reads once,
//! the pre-draw upkeep (fonts, diff stats, selectors), and which pane takes
//! keyboard focus. None of it builds elements.

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::Window;
use gpui_kit::component::WindowExt;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::agent_row::AgentRow;
use crate::workspace_window::pane_focus;
use crate::workspace_window::workspace_title;

impl WorkspaceWindow {
    /// What a frame reads out of the store in one lock - the window's own
    /// title and the rows the sidebar draws - or `None` when this window's
    /// workspace is gone, which is a window that can only say so.
    ///
    /// The title rides along rather than being resolved separately so the
    /// two are read from the same lock scope, and so the title bar cannot
    /// disagree with the rows about which workspace this window is.
    pub(super) fn frame_snapshot(&self, cx: &App) -> Option<(String, Vec<AgentRow>)> {
        let autopilot_enabled = crate::settings_global::read(cx).autopilot_enabled;
        // Evaluating a schedule is arithmetic on the clock, not I/O (#404).
        let now = jiff::Zoned::now();
        let outcomes = cx.try_global::<crate::scheduled_starts::ScheduleOutcomes>();
        let store = self.store.lock();
        let title = workspace_title(&store, self.workspace_id)?;
        let workspace = store.workspaces()
                             .iter()
                             .find(|workspace| workspace.id == self.workspace_id)?;
        Some((title,
              workspace.agent_ids
                       .iter()
                       .filter_map(|id| store.agent(*id))
                       .map(|agent| {
                           let persona_name = agent.persona_id.and_then(|id| {
                                                                  crate::settings_global::read(cx)
                                                       .personas
                                                       .iter()
                                                       .find(|persona| persona.id == id)
                                                       .map(|persona| persona.name.clone())
                                                              });
                           AgentRow { id: agent.id,
                                      avatar: agent.avatar.clone(),
                                      name: agent.name.clone(),
                                      folder: agent.folder.clone(),
                                      state: agent.state,
                                      idle_since: agent.idle_since,
                                      input_reason: agent.input_reason().map(str::to_owned),
                                      session_limit: agent.active_session_limit().cloned(),
                                      is_shell: agent.is_shell(),
                                      is_companion: agent.is_companion,
                                      is_locked: agent.locked,
                                      autopilot: agent.autopilot_applies(autopilot_enabled),
                                      header_title: agent.header_title().to_string(),
                                      persona_name,
                                      agent_type: agent.agent_type.clone(),
                                      is_running: agent.activated,
                                      schedule_line: if agent.is_shell() {
                                          None
                                      }
                                      else {
                                          crate::schedule_text::schedule_line(
                                              agent.schedule.as_ref(),
                                              outcomes.and_then(|outcomes| outcomes.0.get(&agent.id).copied()),
                                              &now,
                                          )
                                      },
                                      idle_line: self.agent_idle
                                                     .get(&agent.id)
                                                     .and_then(|idle| idle.countdown)
                                                     .map(crate::schedule_text::idle_line) }
                       })
                       .collect()))
    }

    /// The work a frame does before it draws: settle the terminal's font
    /// family and hand it to the terminal view, ask for diff stats that have
    /// aged out, and make sure something holds focus.
    ///
    /// None of it draws, none of it runs `git` here - `refresh_diff_stats`
    /// is a map lookup and an `Instant` compare, with the subprocess behind
    /// it running at most every `DIFF_STATS_MAX_AGE` - and none of it asks
    /// the text system which fonts exist: `refresh_terminal_font` is a string
    /// compare unless the configured name changed (see `terminal_font`).
    pub(super) fn prepare_frame(&mut self, is_dashboard: bool, window: &mut Window,
                                cx: &mut Context<Self>) {
        // Ahead of `sync_terminal_style`, the frame's first reader of it.
        self.refresh_terminal_font(cx);
        // The view sizes itself to its bounds; only the font is the window's
        // to hand it.
        self.sync_terminal_style(cx);
        if let Some(id) = self.selected_agent {
            let folder = self.store
                             .lock()
                             .agent(id)
                             .map(|agent| agent.effective_folder().to_owned());
            if let Some(folder) = folder {
                self.refresh_diff_stats(id, &folder);
            }
        }
        if is_dashboard {
            self.refresh_dashboard_diff_stats();
        }
        // The model and effort dropdowns' state, built here because it
        // needs a `&mut Window` the render path does not carry and must
        // outlive the frame that draws it - a state rebuilt per render
        // loses the search query as it is typed. Cheap on the frames that
        // change nothing: the declared values are compared before anything
        // is replaced. The options come from the Panel-mode session's slot,
        // the same state `render_panel_pane` draws the control bar from -
        // not `panel_states`, which belongs to Terminal-mode sessions and
        // never holds a declared option (#451).
        if !is_dashboard && let Some(id) = self.selected_agent {
            let config_options = self.panel_sessions
                                     .get(&id)
                                     .map(|slot| slot.lock().config_options())
                                     .unwrap_or_default();
            self.ensure_panel_selectors(id, &config_options, window, cx);
        }

        self.focus_showing_pane(is_dashboard, window, cx);
        self.key_hints_follow_activation(window);

        // See `root_focus`: without this the Agents menu's items are never
        // on the dispatch path macOS validates them against. Done here
        // rather than beside the element it focuses, because the agent rows
        // built below borrow `cx` until the tree is assembled.
        //
        // After `focus_showing_pane`, which may have taken focus for a
        // composer or a terminal surface already - and then this does
        // nothing, correctly: the menu handlers are declared on the root
        // element and both panes are its descendants.
        //
        // That ordering is also why the terminal's target is gated on a live
        // grid. Focus taken over the "Starting terminal…" placeholder lands
        // on a handle no element tracks, so this line moves focus to the
        // root on the same frame - with the latch already stored, and
        // nothing left to retry.
        if window.focused(cx).is_none() {
            window.focus(&self.root_focus.clone(), cx);
        }
    }

    /// Gives the selected agent's input keyboard focus on the frame its pane
    /// first appears on - its prompt input per `acp-panel-ui`'s "Selecting a
    /// Panel-mode agent focuses its prompt input", or its terminal surface
    /// per `terminal-input`'s "Selecting a Terminal-mode agent focuses its
    /// terminal surface".
    ///
    /// The comparison is against the target the frame is about to *show*,
    /// not against where focus actually is. That is what keeps focus from
    /// being pulled back: once this has focused an agent's input, no later
    /// frame showing the same agent compares differently, however many times
    /// the window redraws or wherever the user has since clicked.
    fn focus_showing_pane(&mut self, is_takeover: bool, window: &mut Window,
                          cx: &mut Context<Self>) {
        // Read before the store lock below rather than inside it: the
        // session's own mutex has no ordering relationship with the store's,
        // and this is not the place to invent one.
        let has_live_grid = self.selected_agent
                                .is_some_and(|id| self.session_has_grid(id));
        let selected = self.selected_agent.and_then(|id| {
                                              let store = self.store.lock();
                                              let agent = store.agent(id)?;
                                              Some(pane_focus::SelectedAgentFacts {
                        id,
                        is_panel_mode: agent.view_mode == knot_core::ViewMode::Panel,
                        is_activated: agent.activated,
                        has_live_grid,
                    })
                                          });
        let showing = pane_focus::focus_target(is_takeover, selected.as_ref());

        // Stored whether or not focus is taken below, so a frame skipped for
        // an open dialog is not replayed as a transition once it closes -
        // the dialog's own scenario is that focus stays with the dialog, and
        // by then the selection is no longer news.
        let changed = showing != self.focused_pane;
        self.focused_pane = showing;
        if !changed {
            return;
        }
        // A takeover draws neither the composer nor the terminal, so a focus
        // handle left on either points at nothing rendered, and gpui resolves
        // that to the dispatch-tree root - above the root element carrying
        // the shortcut and menu handlers. Leaving a panel by ⌥⌘1, or reading
        // the View menu's state, would then find no handler. Focusing the
        // root element keeps them on the path.
        let Some(target) = showing
        else {
            if is_takeover && !window.has_active_dialog(cx) {
                window.focus(&self.root_focus.clone(), cx);
            }
            return;
        };
        // Ask whether a dialog is open rather than where focus sits. Whether
        // a dialog's focus is inside `root_focus`'s subtree is a property of
        // gpui-component's layering, and it has already flipped once - inside
        // through 0.6, outside since 0.7 put the layer in a `Root` plugin.
        // `tests/pane_focus.rs` pins the current answer.
        if window.has_active_dialog(cx) {
            return;
        }
        // After the latch above, never before it: an expanded artifact panel
        // takes the content area, so there is no composer or terminal surface
        // on screen to focus - but collapsing it is not a change of
        // selection, and a guard that fed `focus_target` would latch `None`
        // and then take focus on the transition the collapse produces. Same
        // shape as the dialog guard for the same reason.
        //
        // Every call `prepare_frame` makes before `focus_showing_pane` builds
        // or reconciles state; none of them may be skipped while expanded.
        if self.artifact_panel_expanded(target.agent()) {
            return;
        }
        match target {
            pane_focus::FocusTarget::Composer(id) => {
                let input = self.panel_prompt_input(id, window, cx);
                input.update(cx, |state, cx| state.focus(window, cx));
            }
            pane_focus::FocusTarget::Terminal(id) => {
                self.focus_terminal(id, window, cx);
            }
        }
    }

    /// Whether `id`'s session has produced a grid, which is what decides
    /// between the terminal surface and the "Starting terminal…"
    /// placeholder in `render/content.rs`.
    fn session_has_grid(&self, id: Uuid) -> bool {
        self.terminal_panes.contains_key(&id)
    }
}

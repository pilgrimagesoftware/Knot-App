//! The workspace window entity: what one holds for the life of a window, and
//! what it tears down when the window closes.
//!
//! Every sibling module in `workspace_window` hangs its `impl` off this
//! struct, so the fields are `pub(super)` - visible across the module, not
//! beyond it.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use gpui_kit::AnyWindowHandle;
use gpui_kit::Entity;
use gpui_kit::Subscription;
use gpui_kit::component::resizable::ResizableState;
use gpui_kit::component::select::SelectState;
use gpui_terminal::PtyTransport;
use knot_terminal::TerminalSession;
use parking_lot::Mutex;
use uuid::Uuid;

use super::pane_focus::FocusTarget;
use super::panel;
use super::prompt_queue::QueuedPanelPrompt;
use super::terminal_font::TerminalFont;
use super::view_mode::WorkspaceViewMode;
use crate::composer_style::ComposerStyling;
use crate::dashboard;
use crate::panel_session;
use crate::panel_state;
use crate::panel_view::PanelList;

/// A delivery result on its way back from the runtime: the agent whose
/// queue it belongs to, the queue entry it answers, and how the prompt
/// fared. The entry is named by id rather than by its text so two prompts
/// that read the same do not collect each other's results.
pub(super) type PanelPromptResult = (Uuid, Uuid, Result<(), String>);

pub(crate) struct WorkspaceWindow {
    /// Last known diff stat per agent, refreshed off the render path - see
    /// `refresh_diff_stats`.
    pub(super) diff_stats:                       crate::diff_stats::DiffStatsCache,
    /// Last known state per recorded pull request URL, refreshed off the
    /// render path and only while the Pull Requests view is showing - see
    /// `refresh_pull_request_states`. Never persisted: a merged pull request
    /// shown as open after a restart is worse than a blank.
    pub(super) pull_request_states:              crate::pull_request_state::PullRequestStateCache,
    /// What the last `gh` probe found, and so which single message the Pull
    /// Requests view shows. Probed once per view opening rather than once
    /// per row.
    pub(super) forge_status:                     crate::pull_request_state::ForgeStatus,
    /// Set when a pull request could not be handed to a browser, so the view
    /// can say so. A click that silently did nothing reads as a broken row.
    pub(super) pull_request_open_failed:         bool,
    /// The Pull Requests view's search, filters and sort, for as long as the
    /// window is open. Never persisted: a relaunch opens the view clean.
    pub(super) pull_request_view:                super::pull_requests_actions::PullRequestViewState,
    /// The Changes view's tab and the Issues and OpenSpec tabs' state.
    pub(super) changes_view:                     super::changes_tab::ChangesViewState,
    /// The Issues and OpenSpec tabs' repositories, issues and changes,
    /// fetched off the main thread and drained by `repaint_poll_tick`.
    pub(super) work_items:                       super::work_items::WorkItemCaches,
    /// Codex's usage allowance, read from its session files off the main
    /// thread while a Codex agent is in the window (#291).
    pub(super) codex_usage:
        crate::refresh_cache::RefreshCache<(), Option<crate::codex_usage::CodexUsage>>,
    /// The reset the user dismissed the usage banner for: hidden until the
    /// window resets, then shown again if the next one fills up.
    pub(super) codex_usage_dismissed:            Option<Option<std::time::SystemTime>>,
    /// How Codex's usage is read: `codex_usage::read_current` - this
    /// machine's own `~/.codex` - everywhere but a test, which swaps in a
    /// stub rather than read the developer's files.
    pub(super) codex_usage_source:               fn() -> Option<crate::codex_usage::CodexUsage>,
    /// Agents whose PTY process has exited, queued by the reader thread and
    /// drained by the repaint poll - the callback runs off the main thread
    /// and cannot touch the view directly.
    pub(super) exited_sessions:                  Arc<Mutex<Vec<Uuid>>>,
    /// Keeps the window-bounds observer alive for this window's lifetime.
    pub(super) window_bounds_subscription:       Option<gpui_kit::Subscription>,
    /// Logs this window's close (#458), kept for as long as the view lives.
    /// A close logged with no release after it is a view something still
    /// holds - the leak #457 fixed.
    pub(super) _window_closed:                   Option<gpui_kit::Subscription>,
    /// Which config selector's popover is open, by element id, or `None`
    /// when none is. One shared flag used to back all three: because every
    /// selector's `on_open_change` wrote it and the permission selector
    /// read it, clicking Model or Effort opened the *permission* menu.
    pub(super) open_config_selector:             Option<&'static str>,
    /// The composer's Insert Prompt picker, when one is open - at most one,
    /// for the composer it was opened from.
    pub(super) prompt_picker:                    Option<super::panel::input::PromptPicker>,
    /// Whether the New Agent button's bench popover is open.
    pub(super) bench_popover_open:               bool,
    /// The bench popover row under the pointer, which highlights and
    /// brightens its remove control.
    pub(super) bench_popover_hovered:            Option<Uuid>,
    pub(super) store:                            Arc<Mutex<knot_agents::AgentStore>>,
    /// Agent-to-agent messages, for the unread badge and the idle-time
    /// delivery nudge (`mcp-messaging`). Shared with the MCP server, which
    /// is what writes to it.
    pub(super) messages:                         Arc<Mutex<knot_messaging::MessageStore>>,
    /// The last message each agent has been nudged about, so an unread
    /// inbox produces one prompt rather than one per idle poll.
    pub(super) nudged_messages:                  BTreeMap<Uuid, Uuid>,
    /// The handoff note a restart into a new conversation left for each
    /// agent, taken by the next panel connect and appended to its first
    /// turn (#405). Written and read by this window only, so it lives here
    /// rather than on the shared `Agent`.
    pub(super) panel_handoffs:                   BTreeMap<Uuid, String>,
    /// The `AgentStore::revision` this window last drew, so a status text or
    /// state written where no notify can reach - the MCP server's thread -
    /// still repaints it (#17). Per window: every window polls the one store.
    pub(super) drawn_store_revision:             u64,
    /// Agents the repaint poll has already started because they were
    /// activated without a session (#316, #320) - so one that cannot start
    /// is not retried every tick. See `activation`.
    pub(super) start_requested:                  std::collections::BTreeSet<Uuid>,
    /// The last awaiting-input message each agent was notified about.
    ///
    /// `Effect::AwaitingInput` fires on every status event reporting Input,
    /// not only on the transition into it, so a prompt the user has not
    /// answered keeps arriving. This is what makes the second one a repeat
    /// rather than news, per `desktop-notifications`' suppression rule.
    pub(super) notified_awaiting:                BTreeMap<Uuid, String>,
    pub(super) workspace_id:                     Uuid,
    /// The divider drag in progress, if any (`workspace-layouts`,
    /// "Dividers"). Cleared on release, which is also what persists the
    /// ratios; see `layout`.
    pub(super) divider_drag:                     Option<super::layout::DividerDrag>,
    /// The workspace's layout as this frame read it, taken by
    /// `sync_selection_with_layout` and consumed by the layout area, so the
    /// selection and the panes come from one read of the store. Two reads
    /// could straddle a change from another thread.
    pub(super) frame_layout:                     Option<knot_core::WorkspaceUiState>,
    pub(super) selected_agent:                   Option<Uuid>,
    pub(super) sessions: BTreeMap<Uuid, Arc<Mutex<TerminalSession<PtyTransport>>>>,
    /// The view drawing each session in `sessions`, created and dropped with
    /// it. See `terminal_pane`.
    pub(super) terminal_panes:                   BTreeMap<Uuid, super::terminal_pane::TerminalPane>,
    pub(super) panel_states: BTreeMap<Uuid, Arc<Mutex<panel_state::PanelState>>>,
    /// `TerminalSession::spawn_pty` runs `tokio::spawn` for the activity
    /// tracker; the UI thread has no tokio runtime of its own, so enter
    /// this one around each spawn (see `ensure_session`). Its drop is
    /// bounded - see `window_runtime`.
    pub(super) runtime:                          crate::window_runtime::WindowRuntime,
    /// Backs the divider between the sidebar and the content column.
    ///
    /// The window owns it rather than letting the group keep its own keyed
    /// state inside the element tree: the compact predicate reads the
    /// sidebar's width from outside the group, and a window-held entity gives
    /// it the same source the divider writes.
    pub(super) sidebar_resize:                   Entity<ResizableState>,
    /// Focus target for the window's root element.
    ///
    /// Nothing else in this window claims focus until the user clicks a
    /// pane, and a window with focus nowhere is why the Agents menu drew
    /// disabled with an agent selected: macOS validates each item against
    /// the dispatch path to the focused node, and gpui resolves "no focus"
    /// to the dispatch-tree *root*, which sits above the element carrying
    /// those handlers. Focusing the root element puts them back on the
    /// path, and leaves them there once a pane takes focus, since the root
    /// is that pane's ancestor.
    pub(super) root_focus:                       gpui_kit::FocusHandle,
    /// Whether ⌘ is held, and the sidebar's key hints once it has been held
    /// long enough (`agent-list-ui`).
    pub(super) key_hints:                        super::key_hints::KeyHintHold,
    /// The family the terminal draws and measures in, resolved from the
    /// installed families once per configured name rather than once per
    /// frame - see `terminal_font`.
    pub(super) terminal_font:                    TerminalFont,
    /// Live ACP connections for Panel-mode agents, keyed by agent id -
    /// independent of `sessions` (the terminal PTYs), per the
    /// `acp-panel-ui` "Switch to Terminal mid-turn" scenario: an entry
    /// here persists across a view-mode toggle, only stopped on restart.
    pub(super) panel_sessions: BTreeMap<Uuid, Arc<Mutex<panel_session::PanelSessionSlot>>>,
    /// Every agent's subagents - the ones it dispatched itself, as opposed
    /// to the OS processes `agent_processes` samples.
    ///
    /// Shared rather than owned: the ACP feed writes to it from each panel
    /// session's drain task, and the hook route writes to it from an axum
    /// worker. Neither has a GPUI context, so both leave a changed flag for
    /// `repaint_poll_tick` to drain - the same shape `pull_request_states`
    /// uses, and for the same reason.
    pub(super) subagents: Arc<Mutex<knot_subagents::registry::SubagentRegistry>>,
    /// The process's MCP credentials: panel launches issue an agent's pair
    /// into it and retiring the panel session revokes it (D8).
    pub(super) mcp_credentials:                  crate::mcp_credentials::McpCredentials,
    /// This workspace's agents whose MCP requests have been refused since
    /// they last authenticated, as the repaint poll last read them; drawn
    /// as a notice over their panes (D10).
    pub(super) mcp_unavailable:                  std::collections::BTreeSet<Uuid>,
    /// The lifecycle phase each panel session was in the last time the
    /// repaint poll looked, so a slot moving between phases repaints - see
    /// `panel_needs_repaint`.
    pub(super) panel_phases:                     BTreeMap<Uuid, panel_session::PanelPhase>,
    /// Which spinner frame the working indicators were last repainted on -
    /// see `spinner_repaint_due`.
    pub(super) last_spinner_frame:               u128,
    /// What this window last *focused* - an agent's composer or its terminal
    /// surface - or `None` when the last frame showed neither; see
    /// `prepare_frame`.
    ///
    /// It records what focus was taken for, not where focus is now. Those
    /// differ the moment the user clicks anything else, and that is the
    /// point: the frame compares this against the target it is about to
    /// show, so focus is taken once on the transition into an agent and
    /// never pulled back while the user is working elsewhere in the window.
    /// Reading where focus actually is would undo that.
    ///
    /// One latch over both targets rather than one each, so switching
    /// between agents of different modes is a transition for the one being
    /// switched to - see `pane_focus`.
    pub(super) focused_pane:                     Option<FocusTarget>,
    /// One prompt-entry input per Panel-mode agent that has been viewed,
    /// created lazily. Not part of `Agent`/persistence - purely UI state.
    /// A `Textarea` (not a single-line `Input`) so the expand/collapse
    /// control can grow the same entity's visible height without losing
    /// in-progress text, rather than swapping to a second entity.
    pub(super) panel_prompt_inputs: BTreeMap<Uuid, Entity<panel::prompt::PanelInputState>>,
    /// Each composer's open inputs sheet (`prompt-library`: "Inserting a
    /// prompt with inputs asks for them"): at most one per agent. Pruned in
    /// `teardown_session`, which hands the sheet to
    /// `orphaned_prompt_inputs_sheets` to be closed.
    pub(super) prompt_inputs_sheets:
        BTreeMap<Uuid, gpui_kit::WindowHandle<gpui_kit::component::Root>>,
    /// Sheets whose composer was torn down, waiting for the repaint poll to
    /// close them. `teardown_session` has no `cx` to close a window with,
    /// and a close that waited for the next notify would ride on someone
    /// else's - the poll runs on a timer whether anything notifies or not.
    pub(super) orphaned_prompt_inputs_sheets:
        Vec<gpui_kit::WindowHandle<gpui_kit::component::Root>>,
    /// The model and effort dropdowns' state, one per panel and axis.
    ///
    /// `SelectState` holds the search query, the scroll offset and focus, so
    /// it cannot be rebuilt per render - a state built in the render path
    /// would lose each keystroke as it was typed. Built and refreshed in
    /// `prepare_frame`; see `panel::input::config_select`.
    pub(super) panel_selectors: BTreeMap<panel::input::SelectorKey,
                                         Entity<SelectState<panel::input::ConfigSelectorDelegate>>>,
    /// Keeps each dropdown's `SelectEvent` subscription alive. Dropping one
    /// unsubscribes it, so a selection would persist nothing.
    pub(super) panel_selector_subscriptions:     BTreeMap<panel::input::SelectorKey, Subscription>,
    /// What each dropdown was last built from, so an agent re-reporting the
    /// same options leaves a half-typed search query alone and a changed
    /// list still replaces what is offered.
    pub(super) panel_selector_items:
        BTreeMap<panel::input::SelectorKey, Vec<panel::input::ConfigSelectorItem>>,
    /// Keeps each prompt input's `PressEnter` subscription alive for the
    /// life of the entity it was created for (dropping a `Subscription`
    /// cancels it).
    pub(super) panel_prompt_input_subscriptions: BTreeMap<Uuid, Subscription>,
    pub(super) panel_prompt_queues:              BTreeMap<Uuid, Vec<QueuedPanelPrompt>>,
    /// Each agent's idle tracking (#404); see `workspace_window::idle`.
    /// Pruned in `teardown_session`.
    pub(super) agent_idle:                       BTreeMap<Uuid, super::idle::AgentIdle>,
    /// When idling was last checked, so the check runs on its own interval.
    pub(super) idle_checked_at:                  Option<std::time::Instant>,
    /// The prompt each agent's pump has taken off its queue and is waiting
    /// on a result for.
    ///
    /// Held outside the queue so the row disappears as soon as the prompt is
    /// sent. Held at all for two reasons: the pump must not send the next
    /// prompt before the session reports its turn active, and a failed
    /// delivery goes back on the queue as the same prompt.
    pub(super) panel_prompts_in_flight:          BTreeMap<Uuid, QueuedPanelPrompt>,
    /// One activity tracker per Panel-mode agent whose session has been
    /// ready at least once, created lazily by `sync_panel_agent_states`.
    ///
    /// The tracker, not this window, writes the agent's `AgentState`: its
    /// `on_status` sink does the store write, the same way the hook route's
    /// tracker does in `knot-mcp-tools`. The poll reports ACP transitions
    /// into it and reads nothing back. That is what makes
    /// `Effect::AwaitingInput` (the desktop notification) and
    /// `Effect::CheckMessages` (the idle delivery nudge) reachable for a
    /// Panel-mode agent at all - written straight into the store they never
    /// fired. See `openspec/specs/activity-detection/spec.md`, "ACP updates
    /// drive status for Panel-mode agents".
    pub(super) panel_trackers:                   BTreeMap<Uuid, knot_activity::Tracker>,
    /// The status last *reported* to each agent's tracker, which is what the
    /// poll dedupes against.
    ///
    /// Not the store: the store is now written by the tracker's sink, a
    /// channel hop later, so on the next tick it may still hold the previous
    /// status. Deduping against it would report the same pending permission
    /// twice and raise two notifications for one prompt. Written here
    /// synchronously at send time, so the gate's input is a value this
    /// window owns and nothing off-thread can lag.
    pub(super) panel_reported_states:            BTreeMap<Uuid, knot_agents::AgentState>,
    /// Set by every Panel tracker's `on_status` sink once it has written the
    /// store, and cleared by `repaint_poll_tick` when it reads it.
    ///
    /// The sink runs on the tracker's tokio task with no GPUI context, so
    /// without this the write lands on no frame: the tick that *reports* a
    /// transition notifies while the store still holds the old status, and
    /// the tick the write actually arrives on has nothing to report. The dot
    /// would then catch up only when something unrelated repainted the
    /// window - the failure `.claude/rules/rust-structure.md` names under
    /// "Off-thread results must reach a frame".
    pub(super) panel_status_landed:              Arc<AtomicBool>,
    pub(super) panel_stopping:                   BTreeSet<Uuid>,
    pub(super) panel_prompt_results:             Arc<Mutex<Vec<PanelPromptResult>>>,
    /// Steers' answers, landed off the main thread and drained by the
    /// repaint poll (#26).
    pub(super) panel_steer_results:
        Arc<Mutex<Vec<crate::workspace_window::panel::steer::PanelSteerResult>>>,
    /// One virtualized conversation list per Panel-mode agent that has
    /// been viewed, created lazily - the `ListState` backing
    /// `render_panel`'s virtualization, and the target of the response
    /// action bar's scroll-to-user/scroll-to-top controls and the track
    /// toggle's auto-scroll - with the Markdown state of its replies, which
    /// lives exactly as long as the list (#246).
    pub(super) panel_lists:                      BTreeMap<Uuid, PanelList>,
    /// The item count each `panel_lists` entry was last reconciled to, so
    /// `render_panel_pane` can `splice` only the rows that actually
    /// changed and leave off-screen rows' measured heights alone.
    pub(super) panel_list_row_counts:            BTreeMap<Uuid, usize>,
    /// Each agent's open find bar over its conversation (#251).
    pub(super) panel_finds:                      BTreeMap<Uuid, panel::find_bar::PanelFind>,
    /// Each agent's question card inputs and focus, while one is on screen
    /// (#430).
    pub(super) panel_questions:                  BTreeMap<Uuid, panel::question::QuestionCardUi>,
    /// Files/images attached via the input area's add-context control,
    /// pending the next send - cleared once the prompt is submitted.
    pub(super) panel_pending_context:            BTreeMap<Uuid, Vec<PathBuf>>,
    /// References waiting to be written into a composer. Attaching
    /// context can complete without a window - the add-context control
    /// finishes after its picker closes - and editing a buffer needs one,
    /// so the insertion is deferred to the next frame that has it.
    pub(super) panel_pending_attachments:        BTreeMap<Uuid, Vec<PathBuf>>,
    /// Each Panel-mode agent's file listing for the `@` lookup: how far
    /// along it is, what it found, and the watch following its folder.
    /// Built on the agent's first `@`, since an agent nobody mentions a
    /// file to should not cost a walk - see `panel::mentions`.
    pub(super) panel_mentions:                   BTreeMap<Uuid, panel::mentions::PanelMentions>,
    /// The dashboard cards' quick-prompt fields (#10).
    pub(super) dashboard_quick_prompts:          crate::dashboard_quick_prompt::QuickPromptFields,
    /// The open file finder, if any (#5). One per window, for the selected
    /// agent.
    pub(super) file_finder:                      Option<super::file_finder::FileFinder>,
    /// Each Panel-mode composer's styled runs: its three decoration
    /// collections, the buffer they describe and the palette they were
    /// painted from. Created with the composer entity, so a restored draft
    /// arrives styled; see `panel::styling`.
    pub(super) panel_composer_styling:           BTreeMap<Uuid, ComposerStyling>,
    /// Live `!` commands, keyed by the id of the card drawing each one.
    ///
    /// Not keyed by agent: a panel may have several commands running at
    /// once, each finishing on its own. An entry is removed the poll after
    /// its run settles, by which point the card holds everything the
    /// conversation needs - see `panel::shell`.
    ///
    /// Shared rather than owned outright because the cancel control is a
    /// render closure with no `Context` to reach the window through - the
    /// same reason a panel's session slot is an `Arc`.
    pub(super) panel_shell_runs: Arc<Mutex<BTreeMap<Uuid, panel::shell::PanelShellRun>>>,
    /// Panel-mode agent ids whose input area is expanded to the larger
    /// multi-line editing size; absence means collapsed (the default).
    pub(super) panel_input_expanded:             BTreeSet<Uuid>,
    /// The slash lookup's state per Panel-mode agent: the memoized
    /// command/skill registry, which entry is selected, and the token Esc
    /// closed it on. Created on the agent's first lookup, since building it
    /// reads skill roots off disk - see `panel::lookup`.
    pub(super) panel_lookups:                    BTreeMap<Uuid, panel::lookup::PanelLookup>,
    /// This window's handle, so the poll can tell whether it is the active
    /// window before replacing the app-wide menu bar - two open workspace
    /// windows must not fight over whose selection the Agents menu shows.
    pub(super) window_handle:                    AnyWindowHandle,
    /// The last name written to this window's OS title, so `render` can skip
    /// a `set_window_title` that would change nothing.
    ///
    /// A cache of an *output*, not a copy of the state: the title bar and the
    /// OS title both come from the store every frame, so if this ever drifts
    /// the cost is a redundant AppKit call, never a wrong name. That is what
    /// separates it from the settings snapshot in issue #238, where the copy
    /// *is* what gets read and written back.
    pub(super) titled_as:                        String,
    /// The processes section's state per agent that has one: whether it is
    /// open, the last sample, the last failure, and any termination in
    /// flight. One struct per agent rather than a map per field, so teardown
    /// has one entry to prune.
    pub(super) process_sections: BTreeMap<Uuid, crate::agent_processes::ProcessSection>,
    /// Where the sampling task publishes, and the main thread drains.
    pub(super) process_publish:                  crate::agent_processes::PublishSlot,
    /// The generation this window has already drained, so a poll tick that
    /// finds the same pass again is not mistaken for news.
    pub(super) process_generation:               u64,
    /// When the last sample was asked for, so the poll - which ticks thirty
    /// times a second - runs `ps` on the sampler's cadence instead.
    pub(super) process_sampled_at:               Option<std::time::Instant>,
    /// Set while a sample is in flight, so a slow `ps` is not asked for
    /// twice. The same discipline `RefreshCache::claim_refresh` applies to
    /// `git diff`.
    pub(super) process_sampling:                 Arc<std::sync::atomic::AtomicBool>,
    /// Terminations that were refused, queued by the blocking task and
    /// drained by the poll - the same off-main-thread hand-off
    /// `exited_sessions` uses. Agent, PID, reason.
    pub(super) process_failures:                 Arc<Mutex<Vec<(Uuid, u32, String)>>>,
    /// The MCP section's state per agent that has one: whether it is open,
    /// whether a probe is wanted, the last inventory and the last failure.
    /// One struct per agent rather than a map per field, so teardown has one
    /// entry to prune.
    pub(super) mcp_sections: BTreeMap<Uuid, crate::workspace_window::mcp_panel::state::McpSection>,
    /// Agents with a probe in flight. The authority for "a probe is
    /// running": a claim frees this on drop, so an unwind cannot leave a
    /// header saying "checking" for the life of the window.
    pub(super) mcp_in_flight: crate::workspace_window::mcp_panel::state::InFlight,
    /// Where a finished probe reports and the poll drains - the same
    /// off-main-thread hand-off `process_failures` uses.
    pub(super) mcp_results: crate::workspace_window::mcp_panel::state::ProbeResults,
    /// Rows with a tool listing in flight (#40), released by the claim's
    /// drop like `mcp_in_flight`.
    pub(super) mcp_tools_in_flight: crate::workspace_window::mcp_panel::tools::ToolsInFlight,
    /// Where a finished tool listing reports and the poll drains.
    pub(super) mcp_tool_results: crate::workspace_window::mcp_panel::tools::ToolsResults,
    /// The git panel's per-agent state: what is open, what was last read, the
    /// watches and in-flight operations behind it. See `git_panel_state`.
    pub(super) git:                              super::git_panel_state::GitPanelState,
    /// The artifact panel's arrangement per agent: width, the split between
    /// its two sections, which of them are collapsed, and whether the panel
    /// is expanded over the content pane.
    ///
    /// View state, not persisted, for the reason `git_panel_width` is not -
    /// and keyed by agent rather than by workspace, which is why
    /// `WorkspaceUiState` is the wrong home even though it persists the rest
    /// of the window's arrangement.
    pub(super) artifact_panel:
        BTreeMap<Uuid, super::artifact_panel::state::ArtifactPanelArrangement>,
    /// The divider between an agent's content and its artifact panel, and the
    /// one between the panel's two sections. Held outside the element tree
    /// for the reason `git_panel_resize` is.
    pub(super) artifact_panel_resize:            BTreeMap<Uuid, Entity<ResizableState>>,
    pub(super) artifact_split_resize:            BTreeMap<Uuid, Entity<ResizableState>>,
    /// What each agent's artifact fields held when this window last saw them.
    /// Compared each poll so a `display-markdown` arriving on the MCP
    /// server's thread reaches a frame.
    pub(super) artifact_drawn: BTreeMap<Uuid, super::artifact_panel::state::ArtifactSnapshot>,
    /// What the user last closed in each agent's artifact panel, for the
    /// header's reopen control. Outlives the arrangement, which goes when the
    /// panel closes - remembering past that is the whole point.
    pub(super) artifact_closed: BTreeMap<Uuid, super::artifact_panel::closed::ClosedArtifacts>,
    /// Each agent's checklist answer fields (#301), dropped with the panel
    /// in `forget_artifact_panel`.
    pub(super) checklist_inputs: BTreeMap<Uuid, super::artifact_panel::checklist::ChecklistInputs>,
    /// Each agent's open markdown file, read off the render path. See
    /// `artifact_panel::document`.
    pub(super) markdown_documents:
        BTreeMap<Uuid, super::artifact_panel::document::MarkdownDocument>,
    pub(super) view_mode:                        WorkspaceViewMode,
    pub(super) dashboard_sort:                   dashboard::DashboardSort,
    /// The sidebar's one error line, for a failure the user caused and can
    /// act on - a sidebar width that could not be saved, or a bench entry
    /// pruned because its folder is gone.
    pub(super) error:                            Option<String>,
    /// Where a dragged agent row would land, and where each row was last
    /// painted - see `sidebar_drag`.
    pub(super) agent_drag:                       super::sidebar_drag::AgentRowDrag,
}

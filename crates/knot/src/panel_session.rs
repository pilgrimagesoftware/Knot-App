//! Manages a Panel-mode agent's live ACP connection: starts
//! `knot_terminal::AcpSession`, drains its event stream into
//! `panel_state::PanelState`, and tracks a dirty flag so the UI poll loop
//! knows when to repaint - mirrors `Grid::take_dirty`'s pattern for the
//! terminal view.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use knot_acp::{PermissionDecision, PermissionRequest, Result as AcpResult};
use knot_agent_launch::AdapterConfig;
use knot_terminal::{AcpSession, ConnectProgress, SessionTarget};
use parking_lot::Mutex;

use crate::panel_state::PanelState;
use crate::subagent_feed::SubagentSink;

mod agent_identity;
mod connect;
mod default_mode;
mod drain;
mod live_adapters;
mod slot;
mod times_store;

pub(crate) use agent_identity::AgentIdentity;
pub use connect::{ConnectRequest, StartupRequest, connect_into};
pub(crate) use live_adapters::claim as claim_adapter;
pub use slot::{PanelPhase, PanelSessionSlot};
pub use times_store::{TimesSaver, TimesStore};

pub struct PanelSessionHandle {
    session:        AcpSession,
    state:          Arc<Mutex<PanelState>>,
    dirty:          Arc<AtomicBool>,
    /// The expanded startup prompt, set by `connect_into` before the slot
    /// is published and taken once by the window into the agent's prompt
    /// queue. See `openspec/specs/agent-launch-command/spec.md`.
    startup_prompt: Option<String>,
}

impl PanelSessionHandle {
    /// Starts the adapter connection and spawns a background task (on the
    /// current tokio runtime) draining its event stream into `state` until
    /// the session ends. `target.mcp` is Knot's own MCP HTTP server and the
    /// agent's credential for it, wired into the session when MCP is enabled.
    pub async fn start(config: &AdapterConfig, target: SessionTarget<'_>,
                       progress: &ConnectProgress, subagents: Option<SubagentSink>,
                       times: Option<TimesStore>)
                       -> AcpResult<Self> {
        // Read before the session opens, so the log is in place before the
        // first replayed entry is folded (#220).
        let restored = match (&times, target.prior_session_id) {
            (Some(store), Some(prior)) => {
                let (store, prior) = (store.clone(), prior.to_owned());
                tokio::task::spawn_blocking(move || store.load(&prior)).await
                                                                       .unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let restoring = target.prior_session_id.is_some();
        let (session, config_options, events) = AcpSession::start(config, target, progress).await?;
        let mut initial_state = if restoring {
            PanelState::restoring(restored)
        }
        else {
            PanelState::new()
        };
        // Asked to restore but given a fresh session: nothing replays, and
        // the old log is not this session's.
        if restoring && !session.resumed() {
            initial_state.times.discard_restore();
        }
        initial_state.config_options = config_options;
        let state = Arc::new(Mutex::new(initial_state));
        let dirty = Arc::new(AtomicBool::new(false));

        let saver = times.map(|store| TimesSaver::new(store, session.session_id()));
        drain::spawn(events,
                     Arc::clone(&state),
                     Arc::clone(&dirty),
                     subagents,
                     saver);

        Ok(Self { session,
                  state,
                  dirty,
                  startup_prompt: None })
    }

    /// The agent's name and version as it reported them on `initialize`.
    pub fn agent_info(&self) -> Option<&knot_acp::AgentInfo> {
        self.session.agent_info()
    }

    pub fn state(&self) -> Arc<Mutex<PanelState>> {
        Arc::clone(&self.state)
    }

    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }

    /// The adapter subprocess's process id - this agent's session root.
    pub fn process_id(&self) -> Option<u32> {
        self.session.process_id()
    }

    /// The startup prompt waiting to be queued, handed over at most once.
    pub fn take_startup_prompt(&mut self) -> Option<String> {
        self.startup_prompt.take()
    }

    /// Whether new events arrived since the last call; clears the flag.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::SeqCst)
    }

    // The window sends through `deliver_panel_prompt`, which also handles
    // queueing; this is the unqueued path, kept as the slot's own API.
    #[allow(dead_code)]
    pub async fn prompt(&self, text: &str) -> AcpResult<()> {
        self.session.prompt(text).await
    }

    /// Records a prompt the user just sent in the conversation state - the
    /// ACP stream itself never echoes it back, so the caller must add it
    /// explicitly before (or independent of) actually sending it.
    pub fn record_user_message(&self, text: String) {
        {
            let mut state = self.state.lock();
            state.push_user_message(text);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// [`Self::record_user_message`] for a prompt whose response follows
    /// only when `follow` says so - see `PanelState::push_prompt`.
    pub fn record_prompt(&self, text: String, follow: bool) {
        {
            let mut state = self.state.lock();
            state.push_prompt(text, follow);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Records a prompt steered into the running turn - see
    /// `PanelState::push_steered_prompt`.
    pub fn record_steered_prompt(&self, text: String) {
        self.state.lock().push_steered_prompt(text);
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Adds a notice to the conversation - see `PanelState::push_notice`.
    pub fn record_notice(&self, text: String) {
        self.state.lock().push_notice(text);
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// A cheap clone of the underlying session, for a caller holding this
    /// handle behind a `Mutex` to `.await` on (e.g. `prompt`) without
    /// keeping the lock held across the await point.
    pub fn session(&self) -> AcpSession {
        self.session.clone()
    }

    /// A `'static` writer into this session's conversation, for the task
    /// that awaits a `prompt` after the handle - and the lock it came
    /// from - has been dropped. Without it a prompt that the agent
    /// refuses (an exhausted quota, a transport failure) has nowhere to
    /// report but stderr.
    pub fn recorder(&self) -> PanelRecorder {
        PanelRecorder { state: Arc::clone(&self.state),
                        dirty: Arc::clone(&self.dirty), }
    }

    pub fn answer_permission(&self, request: &PermissionRequest, decision: PermissionDecision) {
        self.session.answer_permission(request, decision);
        {
            let mut state = self.state.lock();
            state.resolve_input();
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Answers the question card on screen with Submit (`true`) or Skip.
    /// Does nothing when no card is showing, or when Submit is unavailable
    /// (a required field empty, a bound broken) - the button is disabled
    /// then, and this is the same check behind it.
    pub fn answer_question(&self, submit: bool) {
        let answered = {
            let mut state = self.state.lock();
            let Some(question) = state.pending_question()
            else {
                return;
            };
            if submit && !question.can_submit() {
                return;
            }
            let answer = if submit {
                question.submitted()
            }
            else {
                knot_acp::FormAnswer::Skipped
            };
            let request = question.request.clone();
            state.resolve_input();
            (request, answer)
        };
        // Outside the lock: the client hands the answer to the task that
        // replies, which also reports the request resolved.
        self.session.answer_question(&answered.0, answered.1);
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Applies `edit` to the question card on screen, if one is, and marks
    /// the panel for repaint when it changed anything.
    pub fn edit_question(&self, edit: impl FnOnce(&mut crate::panel_state::QuestionState) -> bool) {
        let changed = self.state.lock().pending_question_mut().is_some_and(edit);
        if changed {
            self.dirty.store(true, Ordering::SeqCst);
        }
    }

    /// Turns off auto-scroll for the in-flight response, per the track
    /// toggle's "user scrolls away" scenario.
    pub fn clear_tracking(&self) {
        {
            let mut state = self.state.lock();
            state.clear_tracking();
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Toggles auto-scroll for the in-flight response.
    pub fn toggle_tracking(&self) {
        {
            let mut state = self.state.lock();
            state.toggle_tracking();
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Records the user opening or closing one tool-call card, per
    /// `acp-panel-ui`'s "The user's choice outlives the automatic one"
    /// requirement.
    pub fn toggle_tool_call(&self, id: &str) {
        {
            let mut state = self.state.lock();
            state.toggle_tool_call(id);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Drops one `!` command's pending result, so no later prompt carries it.
    pub fn discard_shell_result(&self, card_id: uuid::Uuid) {
        self.state.lock().discard_shell_result(card_id);
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Opens or closes one compact summary's run of tool calls.
    pub fn toggle_tool_run(&self, head_id: String) {
        self.state.lock().toggle_tool_run(head_id);
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Sets auto-scroll directly, for the scroll-to-latest control.
    pub fn set_tracking(&self, tracking: bool) {
        {
            let mut state = self.state.lock();
            state.set_tracking(tracking);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Applies one Session Config Option selection (permission mode,
    /// model, reasoning effort, ...) and stores the agent's updated list.
    /// Returns an owned `'static` future rather than being `async fn`
    /// itself, so a caller with only `&mut App` (a popup-menu click
    /// handler, not a `Context<Self>` listener) can spawn it without
    /// holding this handle - or the session lock it came from - across
    /// the await point.
    pub fn set_config_option(&self, config_id: String, value: String)
                             -> impl std::future::Future<Output = ()> + Send + 'static {
        let session = self.session.clone();
        let state = Arc::clone(&self.state);
        let dirty = Arc::clone(&self.dirty);
        async move {
            if let Ok(config_options) = session.set_config_option(&config_id, &value).await {
                {
                    let mut state = state.lock();
                    state.config_options = config_options;
                }
                dirty.store(true, Ordering::SeqCst);
            }
        }
    }

    pub async fn stop(&self) {
        self.session.stop().await;
    }

    // As `prompt` above: the window cancels via `stop_panel_prompt`.
    #[allow(dead_code)]
    pub async fn cancel(&self) -> AcpResult<()> {
        self.session.cancel().await
    }
}

/// Writes into a panel conversation without holding its handle: an owned
/// pair of the state and the repaint flag, so a spawned task can report a
/// failure back into the panel the user is looking at.
#[derive(Clone)]
pub struct PanelRecorder {
    state: Arc<Mutex<PanelState>>,
    dirty: Arc<AtomicBool>,
}

impl PanelRecorder {
    /// Shows `text` in the conversation as a failed turn.
    pub fn error(&self, text: String) {
        {
            let mut state = self.state.lock();
            state.push_error(text);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Puts a `!` command's card in the conversation, per
    /// `panel-shell-passthrough`.
    ///
    /// Unlike `record_user_message` this starts no turn: a shell command is
    /// the user's own work and the agent is not answering it.
    pub fn shell_command(&self, card: crate::panel_state::ShellCard) {
        {
            let mut state = self.state.lock();
            state.push_shell_command(card);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod notice_tests;
#[cfg(test)]
mod retire_tests;
#[cfg(test)]
mod tests;

//! Concrete MCP tool catalog exposed to agents.
//!
//! Implements `openspec/specs/mcp-tools/spec.md`.

mod agents;
mod args;
mod autopilot;
mod catalog;
mod checklist;
mod confirm;
mod consts;
mod error;
mod identity;
mod lookup;
mod messaging;
mod panels;
mod repos;
mod responses;
#[cfg(test)]
mod session_limit_tests;
mod tasks;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use knot_activity::{EventSink, Tracker, TrackerConfig, tracking_for};
use knot_agents::{AgentState, AgentStore};
use knot_core::BenchAgent;
use knot_core::SharedSettings;
use knot_discovery::RepoInfo;
use knot_mcp::{
    AgentHookHandler, HookRequest, HookStatus, claude_status, codex_turn_complete, extract_metadata,
};
use knot_messaging::{DeliveryNotifier, MessageStore};
use knot_subagents::recognize::hooks::{
    is_subagent_hook, recognize_hook, subagent_payload_is_complete,
};
use knot_subagents::registry::SubagentRegistry;
use parking_lot::Mutex;
use tokio::sync::watch;
use uuid::Uuid;

pub use crate::autopilot::{
    AutopilotEffect, AutopilotQueue, AutopilotSinks, land, spawn_autopilot,
};
pub use crate::catalog::tool_catalog;
pub use crate::confirm::{ConfirmationQueue, ConfirmationRequest, PendingConfirmation};
pub use crate::error::{Result, ToolError};
use crate::lookup::state_string;
pub use crate::tasks::{GraphStore, plan_mermaid, plan_tasks};

/// Agents waiting on the user, each with the message to notify with (`None`
/// for the generic awaiting-input copy). Drained by the owning workspace
/// window, which raises the desktop notification.
pub type AwaitingInputQueue = Arc<Mutex<Vec<(Uuid, Option<String>)>>>;

/// Recipients a direct `send-message` delivered to while they were not
/// activated, for the app to start. Held as an `Option` because the catalog
/// is constructed in tests and benches with no window to drain it; see
/// `messaging::send_message`.
pub type ActivationQueue = Arc<Mutex<Vec<Uuid>>>;

/// The shared subagent registry, as the catalog holds it.
pub type SubagentRegistryHandle = Arc<Mutex<SubagentRegistry>>;

/// The concrete `ToolCatalog` for the tools in
/// `openspec/specs/mcp-tools/spec.md`. Holds every piece of shared state a
/// handler needs; each `call` locks only what that tool touches.
pub struct McpToolCatalog {
    agents:                Arc<Mutex<AgentStore>>,
    messages:              Arc<Mutex<MessageStore>>,
    notifier:              Arc<dyn DeliveryNotifier + Send + Sync>,
    repos:                 watch::Receiver<Vec<RepoInfo>>,
    bench_agents:          Mutex<Vec<BenchAgent>>,
    settings:              Mutex<Option<SharedSettings>>,
    trackers:              Mutex<HashMap<Uuid, Tracker>>,
    awaiting_input:        Mutex<Option<AwaitingInputQueue>>,
    activation:            Mutex<Option<ActivationQueue>>,
    autopilot:             Mutex<Option<crate::autopilot::AutopilotQueue>>,
    /// Committed task plans, one per owning agent. Runtime state, like the
    /// message queue: not persisted, gone with the process.
    graphs:                Mutex<tasks::GraphStore>,
    /// Where hook-reported subagents are recorded, shared with the windows
    /// that draw them and with the ACP feed that writes to it too.
    ///
    /// `None` in a catalog built without one - the tests, and any embedding
    /// that has no UI - in which case subagent events validate and are
    /// acknowledged but recorded nowhere. That is deliberate: the route's
    /// contract with the poster is about the payload, not about whether
    /// anything downstream happens to be listening.
    subagents:             Option<SubagentRegistryHandle>,
    /// Where calls that need the user's confirmation are sent (D9). `None`
    /// with no UI to ask, and then those calls are refused.
    confirmations:         Mutex<Option<ConfirmationQueue>>,
    /// How long such a call waits for an answer before it is refused.
    confirmation_timeout:  Mutex<std::time::Duration>,
    /// The agents with a call waiting on the user, at most one each.
    awaiting_confirmation: Mutex<std::collections::HashSet<Uuid>>,
}

impl McpToolCatalog {
    /// Takes the same `Arc<Mutex<AgentStore>>` the binary's shell renders, so
    /// MCP-driven mutations (`register-agent`, `set-status`, `create-agent`,
    /// hooks) land in the store the UI reads.
    pub fn new(agents: Arc<Mutex<AgentStore>>, repos: watch::Receiver<Vec<RepoInfo>>,
               notifier: Arc<dyn DeliveryNotifier + Send + Sync>)
               -> Self {
        Self { agents,
               messages: Arc::new(Mutex::new(MessageStore::new())),
               notifier,
               repos,
               bench_agents: Mutex::new(Vec::new()),
               settings: Mutex::new(None),
               trackers: Mutex::new(HashMap::new()),
               awaiting_input: Mutex::new(None),
               activation: Mutex::new(None),
               autopilot: Mutex::new(None),
               graphs: Mutex::new(tasks::GraphStore::new()),
               subagents: None,
               confirmations: Mutex::new(None),
               confirmation_timeout: Mutex::new(consts::CONFIRMATION_TIMEOUT),
               awaiting_confirmation: Mutex::new(std::collections::HashSet::new()) }
    }

    pub fn with_message_store(mut self, messages: Arc<Mutex<MessageStore>>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_awaiting_input_queue(self, queue: AwaitingInputQueue) -> Self {
        *self.awaiting_input.lock() = Some(queue);
        self
    }

    pub fn with_activation_queue(self, queue: ActivationQueue) -> Self {
        *self.activation.lock() = Some(queue);
        self
    }

    /// Sends calls that need the user's confirmation to `queue` (D9).
    pub fn with_confirmation_queue(self, queue: ConfirmationQueue) -> Self {
        *self.confirmations.lock() = Some(queue);
        self
    }

    /// Waits `timeout` for a confirmation instead of the default
    /// [`consts::CONFIRMATION_TIMEOUT`] - for tests.
    pub fn with_confirmation_timeout(self, timeout: std::time::Duration) -> Self {
        *self.confirmation_timeout.lock() = timeout;
        self
    }

    /// Refreshes the bench-agent templates `create-agent`'s `benchAgentId`
    /// resolves against. The catalog has no settings dependency of its own;
    /// the caller (`crates/knot`) pushes updates in whenever settings
    /// change.
    pub fn set_bench_agents(&self, bench_agents: Vec<BenchAgent>) {
        *self.bench_agents.lock() = bench_agents;
    }

    /// A clone of the current agent list, for `McpServer`'s
    /// `AgentsSnapshotFn` (the `GET /api/v1/agent/status` endpoint).
    pub fn agents_snapshot(&self) -> Vec<knot_agents::Agent> {
        self.agents.lock().agents().to_vec()
    }

    /// The settings surface this catalog persists the roster through.
    ///
    /// Takes the shared handle rather than an owned `Settings`. The catalog
    /// lives on the MCP server's own thread for the whole session, so a
    /// snapshot taken when the server started would still be read hours
    /// later - `restore_conversation_on_launch` below is exactly that read,
    /// and it decides what a persisted agent carries. Issue #238 in a
    /// background service.
    pub fn with_settings(self, settings: SharedSettings) -> Self {
        *self.settings.lock() = Some(settings);
        self
    }

    fn persist_agent_state(&self) -> Result<()> {
        let agents = self.agents.lock();
        let settings = self.settings.lock();
        let Some(settings) = settings.as_ref()
        else {
            return Ok(());
        };
        let installed = settings.write(|settings| {
                                    settings.saved_agents =
                                agents.saved_agents(settings.restore_conversation_on_launch);
                                    settings.saved_workspaces = agents.saved_workspaces();
                                });
        Ok(installed.persist_roster()?)
    }

    fn tracker_for(&self, id: Uuid, agent_type: &str) -> bool {
        // Only the types whose hooks report progress get a tracker; for
        // the rest it would arm timers nothing ever feeds.
        if !knot_core::agent_type::has_hook_activity(agent_type) {
            return false;
        }
        let mut trackers = self.trackers.lock();
        if trackers.contains_key(&id) {
            return true;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return false;
        }

        let agents = Arc::clone(&self.agents);
        let reasons = Arc::clone(&self.agents);
        let awaiting_input = self.awaiting_input.lock().clone();
        let sink = EventSink { on_status: Some(Box::new(move |event| {
                                                   agents.lock()
                                                         .set_tracked_state(id, event.status);
                                               })),
                               // The hook's own message - "Claude needs your
                               // permission to use Bash" - is why the agent
                               // waits, on its cell as in the notification
                               // (#344).
                               on_awaiting_input:
                                   Some(Box::new(move |message: Option<String>| {
                                            reasons.lock().set_awaiting_reason(id, message.clone());
                                            if let Some(queue) = &awaiting_input {
                                                queue.lock().push((id, message));
                                            }
                                        })),
                               ..Default::default() };
        // Hook-reported status is exclusively a Terminal-mode path - a
        // Panel-mode agent's status is driven by ACP session events
        // instead, never by this hook tracker.
        let tracker = Tracker::spawn(TrackerConfig { is_hook_based: true,
                                                     ..TrackerConfig::for_agent_type(agent_type) },
                                     tracking_for(agent_type, knot_core::ViewMode::Terminal),
                                     sink);
        trackers.insert(id, tracker);
        true
    }
}

impl AgentHookHandler for McpToolCatalog {
    fn register(&self, request: &HookRequest) -> Result<serde_json::Value, knot_mcp::HookError> {
        if request.agent != "claude" {
            return Err(knot_mcp::HookError::UnknownAgent(request.agent.clone()));
        }
        let id = request.agent_id()?;
        let mut agents = self.agents.lock();
        let Some(agent) = agents.agent(id)
        else {
            return Err(knot_mcp::HookError::InvalidPayload("Agent not found".to_string()));
        };
        let is_resuming = agent.resume_session_id.is_some() && !agent.fork_session;
        let is_fork = agent.fork_session;
        let source = request.source.as_deref().unwrap_or("startup");
        agents.set_registered(id, true);
        agents.update_metadata(id, extract_metadata("claude", &request.payload));
        if source == "resume" {
            if !is_fork && let Some(session_id) = request.session_id.clone() {
                agents.set_session_id(id, session_id);
            }
        }
        else {
            if !is_resuming && let Some(session_id) = request.session_id.clone() {
                agents.set_session_id(id, session_id);
            }
        }
        Ok(serde_json::json!({
            "success": true,
            "message": "Registered",
            "unreadMessageCount": 0,
            "knotMembers": agents.agents().iter().map(|agent| serde_json::json!({
                "id": agent.id.to_string(),
                "name": agent.name,
                "folder": agent.folder,
                "status": state_string(agent.state),
                "isRegistered": agent.is_registered,
            })).collect::<Vec<_>>(),
        }))
    }

    fn status(&self, request: &HookRequest) -> Result<serde_json::Value, knot_mcp::HookError> {
        let id = request.agent_id()?;
        if self.agents.lock().agent(id).is_none() {
            return Err(knot_mcp::HookError::InvalidPayload("Agent not found".to_string()));
        }
        // Before the status vocabulary is consulted, and returning early: a
        // subagent event is keyed on `hook`, carries no `status`, and must
        // not move the agent's activity state as a side effect of being
        // reported. Falling through would reject it as a missing status.
        if let Some(hook) = request.hook.as_deref()
           && is_subagent_hook(hook)
        {
            return self.subagent_event(id, hook, &request.payload);
        }
        self.agents
            .lock()
            .update_metadata(id, extract_metadata(&request.agent, &request.payload));
        let status = match request.agent.as_str() {
            "claude" => claude_status(
                request
                    .status
                    .as_deref()
                    .ok_or_else(|| knot_mcp::HookError::InvalidStatus("missing".to_string()))?,
            )?,
            "codex" => {
                let thread_id = codex_turn_complete(&request.payload)?;
                if let Some(thread_id) = thread_id {
                    self.agents.lock().set_session_id(id, thread_id);
                }
                HookStatus::Idle
            }
            agent => return Err(knot_mcp::HookError::UnknownAgent(agent.to_string())),
        };
        let state = match status {
            HookStatus::Working => AgentState::Running,
            HookStatus::Idle => AgentState::Idle,
            HookStatus::AwaitingInput => AgentState::Input,
        };
        let input_message =
            matches!(status, HookStatus::AwaitingInput).then(|| {
                                                           hook_input_message(&request.payload)
                                                       })
                                                       .flatten();
        // A prompt the user submits is what lifts a session limit seen in
        // the terminal (#36); everything else the hooks report is the
        // wrap-up, and `set_tracked_state` keeps the limit through it.
        if request.hook.as_deref() == Some("UserPromptSubmit") {
            self.agents.lock().set_session_limit(id, None);
        }
        if self.tracker_for(id, &request.agent) {
            self.trackers
                .lock()
                .get(&id)
                .expect("tracker inserted above")
                .apply_hook_status(state, input_message);
        }
        else {
            self.agents.lock().set_tracked_state(id, state);
        }
        // Going idle is this feed's turn-end: the agent has stopped working,
        // so whatever it delegated during the turn is over. The ACP feed has
        // `TurnEnd` for the same job. Without this, a terminal agent's
        // subagents would accumulate across every turn of a session.
        if matches!(state, AgentState::Idle)
           && let Some(subagents) = &self.subagents
        {
            subagents.lock().clear(id);
        }
        // After the state write above: classification runs off this
        // thread and, if it lands on an effect, re-marks the agent
        // `Input` itself - starting from `Idle` here is what that
        // transition is relative to.
        if matches!(state, AgentState::Idle) {
            self.maybe_trigger_autopilot(id,
                                         &request.agent,
                                         request.hook.as_deref(),
                                         &request.payload);
        }
        Ok(serde_json::json!({"success": true}))
    }
}

impl McpToolCatalog {
    /// Shares the window's subagent registry, so hook-reported delegations
    /// land in the same place the ACP feed writes to.
    #[must_use]
    pub fn with_subagents(mut self, subagents: SubagentRegistryHandle) -> Self {
        self.subagents = Some(subagents);
        self
    }

    /// Records one subagent lifecycle event.
    ///
    /// A malformed payload is a 400, per `agent-hooks`. An event naming a
    /// subagent with no recorded dispatch is *not*: Knot may have started
    /// after the dispatch, and answering an error there would make the
    /// agent's hook report a failure for something it did correctly. The
    /// registry drops it silently.
    fn subagent_event(&self, id: Uuid, hook: &str, payload: &serde_json::Value)
                      -> Result<serde_json::Value, knot_mcp::HookError> {
        if !subagent_payload_is_complete(hook, payload) {
            return Err(knot_mcp::HookError::InvalidPayload(
                "Subagent event is missing a required field".to_string(),
            ));
        }

        if let Some(subagents) = &self.subagents
           && let Some(event) = recognize_hook(hook, payload)
        {
            subagents.lock().apply(id, event, Instant::now());
        }

        Ok(serde_json::json!({"success": true}))
    }
}

fn hook_input_message(payload: &serde_json::Value) -> Option<String> {
    payload.get("message")
           .and_then(serde_json::Value::as_str)
           .map(str::to_owned)
}

#[cfg(test)]
mod tests;

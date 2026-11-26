use std::collections::BTreeMap;

use knot_core::{
    ActivationMode, Capabilities, CostTier, SavedPullRequest, StartupPrompt, Workspace,
    WorkspaceUiState,
};
use uuid::Uuid;

use crate::agent::{Agent, AgentState};

mod arrange;
mod bench;
mod checklist;
mod duplicate;
mod edit;
mod fresh_session;
mod helpers;
pub mod layout;
mod lifecycle;
mod mcp_handover;
mod ordering;
mod panels;
mod persistence;
mod pull_requests;

pub use arrange::LayoutChange;
pub use mcp_handover::{McpHandoverRequest, McpHandoverTerminal};
pub use persistence::AdoptedCounts;
mod workspace;

#[cfg(test)]
mod tests;

const DEFAULT_WORKSPACE_NAME: &str = "Knot";
const DEFAULT_WORKSPACE_COLOR: &str = "#1B4FB2";

/// Fields a caller may override when creating an agent; everything else
/// (id, runtime state) is derived. `folder` is passed separately to
/// `AgentStore::create` since it is the one field every creation supplies.
#[derive(Debug, Clone, Default)]
pub struct CreateOptions {
    pub name:             Option<String>,
    pub avatar:           Option<String>,
    pub agent_type:       Option<String>,
    pub shell_command:    Option<String>,
    pub persona_id:       Option<Uuid>,
    pub created_by:       Option<Uuid>,
    pub is_companion:     bool,
    pub insert_after:     Option<Uuid>,
    /// Workspace to place the new agent in. Takes priority over inferring one
    /// from `created_by`/`insert_after`. A caller that knows which workspace
    /// it means must say so explicitly rather than relying on whichever
    /// workspace happens to be "current" - that's ambient global state a
    /// concurrent window or MCP call can change out from under it.
    pub workspace_id:     Option<Uuid>,
    /// Defaults to `Passive` - deliberately not the load default a record
    /// with no stored mode gets (`Active`, see `knot_core::SavedAgent`).
    pub activation_mode:  ActivationMode,
    /// Registry metadata. All three default to "nothing declared", which is
    /// what an agent created without them should read as.
    pub description:      String,
    pub capabilities:     Capabilities,
    pub cost_tier:        CostTier,
    pub startup_prompt:   Option<StartupPrompt>,
    /// Marks the new agent as the terminal handing the user its owner's MCP
    /// flow for this server (#443). Use [`AgentStore::open_mcp_handover`]
    /// rather than setting it here.
    pub mcp_handover:     Option<String>,
    /// For a companion, how it came to be created, which decides what its
    /// creation does to its owner's workspace layout (`workspace-layouts`).
    /// `None` leaves the layout alone, which is what every path that has
    /// not decided yet gets. A caller
    /// that creates a companion should say whether the user asked for it
    /// ([`layout::CompanionOrigin::User`]) or not.
    pub companion_origin: Option<layout::CompanionOrigin>,
}

/// Fields an edit may change. `name`/`avatar` always apply and never trigger
/// a restart; `folder`/`agent_type`/persona changes do.
#[derive(Debug, Clone, Default)]
pub struct EditRequest {
    pub name:                String,
    pub avatar:              String,
    pub folder:              Option<String>,
    pub agent_type:          Option<String>,
    pub persona_id:          Option<Uuid>,
    pub persona_changed:     bool,
    pub relocate_companions: bool,
    /// Applied verbatim; changing it never triggers a restart, per
    /// `agent-lifecycle`'s "Activation mode" requirement.
    pub activation_mode:     ActivationMode,
    /// Registry metadata, applied verbatim. Like `activation_mode`, none of
    /// it triggers a restart: re-tagging an agent says nothing about the
    /// session it already has, and interrupting one to record a label would
    /// throw away the work it is doing.
    pub description:         String,
    pub capabilities:        Capabilities,
    pub cost_tier:           CostTier,
    /// Applied verbatim and restart-free: it is sent on the next fresh
    /// session, not to the one already running.
    pub startup_prompt:      Option<StartupPrompt>,
    /// When the agent starts on its own (#404). Applied verbatim and
    /// restart-free: it takes effect at the next run that comes due.
    pub schedule:            Option<knot_core::schedule::Schedule>,
    /// What idling does to it (#404). Restart-free, read at the next check.
    pub idle_policy:         knot_core::schedule::IdlePolicy,
}

/// An agent removed by [`AgentStore::remove`], in cascade order (companions
/// before their owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovedAgent {
    pub id:             Uuid,
    /// Whether the caller owes MCP an unregister call for this id.
    pub was_registered: bool,
}

/// Owns the agent list and the workspaces that place them, implementing
/// every operation `openspec/specs/agent-lifecycle/spec.md` names.
#[derive(Debug, Clone, Default)]
pub struct AgentStore {
    agents:               Vec<Agent>,
    workspaces:           Vec<Workspace>,
    /// How each workspace's window is arranged, keyed by workspace id.
    ///
    /// A second per-key map on a struct that already has several, which the
    /// parallel-maps rule warns about. Accepted here because the two have
    /// genuinely different lifetimes - a workspace's configuration outlives
    /// any window, and its arrangement is meaningless without one - and
    /// because only this one is rewritten by a pointer drag. The pruning that
    /// keeps them from disagreeing is in `remove_workspace` and, for anything
    /// that removes a workspace by another path, on load in `knot-core`. See
    /// `openspec/specs/settings-persistence/spec.md`.
    workspace_ui:         BTreeMap<Uuid, WorkspaceUiState>,
    current_workspace_id: Option<Uuid>,
    /// The pull requests Knot has seen in these agents' output. Held here
    /// rather than only in the settings document so that removing an agent
    /// or a workspace can take its records with it, in the one place that
    /// knows either is going away. See [`pull_requests`].
    pull_requests:        Vec<SavedPullRequest>,
    /// Counts the changes a window draws but may not have made itself -
    /// an agent's status text and state, written from the MCP server's
    /// thread or a tracker, with no GPUI context to notify from (#17).
    ///
    /// A counter, not a flag: every workspace window polls the one store,
    /// and a flag the first to poll cleared would leave the rest showing
    /// the old text. Each window remembers the revision it last drew
    /// instead. Moved only when a value actually changes, so a write that
    /// repeats what is there does not cost a repaint.
    revision:             u64,
}

impl AgentStore {
    /// How many drawn changes the store has seen; see the `revision` field.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Records a drawn change.
    fn revise(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn saved_workspaces(&self) -> Vec<Workspace> {
        self.workspaces.clone()
    }

    /// The whole UI-state map, for persisting it.
    pub fn saved_workspace_ui(&self) -> BTreeMap<Uuid, WorkspaceUiState> {
        self.workspace_ui.clone()
    }

    pub fn agent(&self, id: Uuid) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub(super) fn agent_mut(&mut self, id: Uuid) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// Apply `f` to the agent with `id`, if the store still holds one.
    ///
    /// Every one-line setter below is this and nothing else. An unknown id
    /// is not an error: an agent can be removed while a message about it is
    /// still in flight, and the update is then simply dropped.
    fn update(&mut self, id: Uuid, f: impl FnOnce(&mut Agent)) {
        if let Some(agent) = self.agent_mut(id) {
            f(agent);
        }
    }

    /// Mark whether this agent may start in this run. Set by selection and
    /// by `create` for an `Active` agent; cleared by deactivation. Runtime
    /// only - never persisted.
    pub fn set_activated(&mut self, id: Uuid, activated: bool) {
        self.update(id, |agent| agent.activated = activated);
    }

    /// Lock or unlock `id` against agent messaging; see [`Agent::locked`].
    /// Moves the revision, so every window redraws the row's lock.
    pub fn set_locked(&mut self, id: Uuid, locked: bool) {
        let changed = self.agent(id).is_some_and(|agent| agent.locked != locked);
        self.update(id, |agent| agent.locked = locked);
        if changed {
            self.revise();
        }
    }

    /// Turn Autopilot off, or back on, for `id` alone; see
    /// [`Agent::autopilot_off`]. Moves the revision, so every window redraws
    /// the agent's indicator.
    pub fn set_autopilot_off(&mut self, id: Uuid, off: bool) {
        let changed = self.agent(id)
                          .is_some_and(|agent| agent.autopilot_off != off);
        self.update(id, |agent| agent.autopilot_off = off);
        if changed {
            self.revise();
        }
    }

    /// Give `id` a schedule, or take it away (#404). Moves the revision, so
    /// every window redraws the agent's clock line. A new or changed schedule
    /// runs only at occurrences after now; see
    /// [`Self::seed_schedule_handled`].
    pub fn set_schedule(&mut self, id: Uuid, schedule: Option<knot_core::schedule::Schedule>) {
        self.set_schedule_at(id, schedule, jiff::Timestamp::now());
    }

    /// [`Self::set_schedule`] at a chosen time, for tests.
    pub fn set_schedule_at(&mut self, id: Uuid, schedule: Option<knot_core::schedule::Schedule>,
                           now: jiff::Timestamp) {
        let changed = self.agent(id)
                          .is_some_and(|agent| agent.schedule != schedule);
        let seed = changed && schedule.is_some();
        self.update(id, |agent| agent.schedule = schedule);
        if seed {
            self.seed_schedule_handled(id, now.as_second());
        }
        if changed {
            self.revise();
        }
    }

    /// Set what idling does to `id` (#404). Moves the revision, as
    /// [`Self::set_schedule`] does.
    pub fn set_idle_policy(&mut self, id: Uuid, policy: knot_core::schedule::IdlePolicy) {
        let changed = self.agent(id)
                          .is_some_and(|agent| agent.idle_policy != policy);
        self.update(id, |agent| agent.idle_policy = policy);
        if changed {
            self.revise();
        }
    }

    pub fn set_registered(&mut self, id: Uuid, registered: bool) {
        self.update(id, |agent| agent.is_registered = registered);
    }

    pub fn set_session_id(&mut self, id: Uuid, session_id: String) {
        self.update(id, |agent| agent.session_id = Some(session_id));
    }

    pub fn apply_acp_session_outcomes(&mut self, outcomes: &BTreeMap<Uuid, Option<String>>) {
        for agent in &mut self.agents {
            if let Some(outcome) = outcomes.get(&agent.id) {
                agent.acp_session_id = outcome.clone();
            }
        }
    }

    /// Set `id`'s state, recording the moment it goes idle in
    /// [`Agent::idle_since`]. Only a transition counts: a repeated `Idle`
    /// keeps the time the agent actually finished, rather than moving it to
    /// whenever the state was last re-reported.
    ///
    /// Leaving [`AgentState::Input`] clears [`Agent::awaiting_reason`]: the
    /// wait it explained is over, whichever path ended it.
    pub fn set_state(&mut self, id: Uuid, state: AgentState) {
        let changed = self.agent(id).is_some_and(|agent| agent.state != state);
        self.update(id, |agent| {
                if state == AgentState::Idle && agent.state != AgentState::Idle {
                    agent.idle_since = Some(std::time::SystemTime::now());
                }
                if state != AgentState::Input {
                    agent.awaiting_reason = None;
                }
                if state != AgentState::Error {
                    agent.session_limit = None;
                }
                agent.state = state;
            });
        if changed {
            self.revise();
        }
    }

    /// Sets `id`'s state as a terminal agent's output tracker or hooks
    /// inferred it (#36).
    ///
    /// While the agent is held by a session limit - in Error with one, or
    /// in Input over one - Working and Idle do not land: the wrap-up a
    /// Claude session is allowed past its limit goes on printing, calling
    /// tools and ends in a Stop, and none of that lifts the limit. They put
    /// the agent back in Error instead. Input does land, keeping the limit
    /// underneath: a permission the wrap-up asks for needs the user more
    /// urgently than a limit they can do nothing about. A prompt the user
    /// submits lifts the limit, by clearing it through
    /// [`Self::set_session_limit`] first.
    pub fn set_tracked_state(&mut self, id: Uuid, state: AgentState) {
        let held = self.agent(id).is_some_and(|agent| {
                                     agent.session_limit.is_some()
                                     && matches!(agent.state, AgentState::Error | AgentState::Input)
                                 });
        if !held {
            self.set_state(id, state);
            return;
        }
        let shown = if state == AgentState::Input {
            AgentState::Input
        }
        else {
            AgentState::Error
        };
        let changed = self.agent(id).is_some_and(|agent| agent.state != shown);
        // Not through `set_state`, which would drop the limit with Error.
        self.update(id, |agent| {
                if shown != AgentState::Input {
                    agent.awaiting_reason = None;
                }
                agent.state = shown;
            });
        if changed {
            self.revise();
        }
    }

    /// Records why `id` is waiting on the user; see
    /// [`Agent::awaiting_reason`]. Set before or after the state moves to
    /// Input - the two arrive from different sinks in either order - and
    /// kept until the agent leaves Input. `None` leaves a reason already
    /// given in place: a wait re-reported without one is the same wait.
    pub fn set_awaiting_reason(&mut self, id: Uuid, reason: Option<String>) {
        let Some(reason) = reason.filter(|reason| !reason.trim().is_empty())
        else {
            return;
        };
        let changed = self.agent(id)
                          .is_some_and(|agent| agent.awaiting_reason.as_deref() != Some(&reason));
        self.update(id, |agent| agent.awaiting_reason = Some(reason));
        if changed {
            self.revise();
        }
    }

    /// Records the usage limit that stopped `id`'s session; see
    /// [`Agent::session_limit`]. Moves the revision on a change, since the
    /// write comes from the window's poll for an agent it may not show.
    pub fn set_session_limit(&mut self, id: Uuid, limit: Option<crate::SessionLimit>) {
        let changed = self.agent(id)
                          .is_some_and(|agent| agent.session_limit != limit);
        self.update(id, |agent| agent.session_limit = limit);
        if changed {
            self.revise();
        }
    }

    /// Asks for `id` to start its next task on a fresh conversation (#331);
    /// see [`Agent::fresh_context_requested`]. [`Self::restart`] carries it
    /// out and clears it. Moves the revision, since the write comes from the
    /// MCP server's thread and the window acts on it.
    pub fn request_fresh_context(&mut self, id: Uuid) {
        let changed = self.agent(id)
                          .is_some_and(|agent| !agent.fresh_context_requested);
        self.update(id, |agent| agent.fresh_context_requested = true);
        if changed {
            self.revise();
        }
    }

    pub fn update_metadata(&mut self, id: Uuid, metadata: BTreeMap<String, String>) {
        self.update(id, |agent| agent.metadata.extend(metadata));
    }

    /// Sets `id`'s status text - the `set-status` tool's write, which lands
    /// on the MCP server's thread. Moves [`Self::revision`] when the text
    /// changes, which is how a window not otherwise repainting finds out.
    pub fn set_status_text(&mut self, id: Uuid, status: String) {
        let changed = self.agent(id)
                          .is_some_and(|agent| agent.status_text != status);
        self.update(id, |agent| agent.status_text = status);
        if changed {
            self.revise();
        }
    }

    /// Sets where `id` says it is working (#25); `None` clears it, and so
    /// does `dir` being the agent's own folder. Revises the store when the
    /// effective folder changes, which is what repaints the windows reading
    /// git from it.
    pub fn set_working_dir(&mut self, id: Uuid, dir: Option<String>) {
        let Some(agent) = self.agent(id)
        else {
            return;
        };
        let dir = dir.filter(|dir| *dir != agent.folder);
        if agent.working_dir == dir {
            return;
        }
        self.update(id, |agent| agent.working_dir = dir);
        self.revise();
    }

    pub fn set_terminal_title(&mut self, id: Uuid, title: String) {
        self.update(id, |agent| agent.terminal_title = title);
    }
}

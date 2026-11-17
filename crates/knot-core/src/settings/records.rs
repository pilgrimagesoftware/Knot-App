//! The serialized record types stored inside [`super::Settings`]: personas,
//! saved agents, bench templates, and workspaces. Field names use the Swift
//! `CodingKeys` (camelCase) so a document written by either implementation
//! round-trips. `#[serde(default)]` on the later-added fields is the
//! decode-tolerant migration path.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::consts::{
    DEFAULT_AGENT_TYPE, DEFAULT_AVATAR, WORKSPACE_LAYOUT_DEFAULT, WORKSPACE_SPLIT_RATIO_DEFAULT,
};
use crate::settings::capabilities::Capabilities;
use crate::settings::prompts::StartupPrompt;
use crate::settings::vocabulary::CostTier;

// ---------------------------------------------------------------------------
// Persona
// ---------------------------------------------------------------------------

/// Origin of a persona: shipped with the app or created by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersonaType {
    System,
    User,
}

/// Lifecycle state. `Deleted` is a soft delete so a removed system persona is
/// not reinstalled on the next launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersonaState {
    Enabled,
    Disabled,
    Deleted,
}

/// A named instruction block an agent can adopt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Persona {
    pub id:           Uuid,
    pub name:         String,
    pub instructions: String,
    #[serde(rename = "type", default = "default_persona_type")]
    pub persona_type: PersonaType,
    #[serde(default = "default_persona_state")]
    pub state:        PersonaState,
}

/// Which surface an agent is driven through: the terminal grid (default,
/// every agent type), or a native ACP panel for agent types with a
/// registered adapter (`knot-agent-launch::acp_adapter`). The terminal is
/// never removed - Panel mode only changes which view/launch path is
/// active, per `openspec/specs/acp-panel-ui/spec.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    #[default]
    Terminal,
    Panel,
}

/// When an agent's session starts on its own: an `Active` agent starts with
/// its workspace, a `Passive` one waits to be selected. See
/// `openspec/specs/agent-lifecycle/spec.md` - "Activation mode".
///
/// `Passive` is the enum's own default because that is what the new-agent
/// dialog offers; the *load* default is deliberately different, see
/// [`default_activation_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    Active,
    #[default]
    Passive,
}

// ---------------------------------------------------------------------------
// SavedAgent
// ---------------------------------------------------------------------------

/// A persisted agent. Runtime-only fields are not stored; loading reconstructs
/// them at their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAgent {
    pub id:              Uuid,
    pub name:            String,
    #[serde(default = "default_avatar")]
    pub avatar:          String,
    pub folder:          String,
    #[serde(default = "default_agent_type")]
    pub agent_type:      String,
    #[serde(default)]
    pub created_by:      Option<Uuid>,
    #[serde(default)]
    pub is_companion:    bool,
    #[serde(default)]
    pub shell_command:   Option<String>,
    #[serde(default)]
    pub persona_id:      Option<Uuid>,
    #[serde(default)]
    pub session_id:      Option<String>,
    #[serde(default)]
    pub view_mode:       ViewMode,
    #[serde(default)]
    pub acp_session_id:  Option<String>,
    /// Loads as `Active`, not the enum's own `Passive` default: a record
    /// written before this field existed describes an agent that started
    /// with its workspace, and it must go on doing so. New agents get
    /// `Passive` from `CreateOptions` instead.
    #[serde(default = "default_activation_mode")]
    pub activation_mode: ActivationMode,
    /// Shut out of agent messaging: it receives no messages, so none starts
    /// it, and it sends none (#310). The user's choice, kept across a
    /// relaunch; absent from older records, which load unlocked.
    #[serde(default)]
    pub locked:          bool,
    /// Autopilot turned off for this agent alone, whatever the global
    /// setting (#323). Absent from older records, which load following it.
    #[serde(default)]
    pub autopilot_off:   bool,
    /// Registry metadata: what this agent is for, what it can be asked to
    /// do, and how expensive it is to ask. Absent from records written
    /// before the registry existed, which load undescribed, untagged and
    /// mid-priced rather than being hidden. See
    /// `openspec/specs/agent-registry/spec.md`.
    #[serde(default)]
    pub description:     String,
    #[serde(default)]
    pub capabilities:    Capabilities,
    #[serde(default)]
    pub cost_tier:       CostTier,
    /// The session setup last chosen in the Panel - model, permission mode
    /// and reasoning effort - keyed by the ACP config-option id the adapter
    /// declared, holding that option's selected value.
    ///
    /// Not an enum per closed-vocabulary convention, and not three named
    /// fields: the ids and their legal values are declared by the adapter at
    /// runtime (`ConfigOption`), not fixed by Knot, and they differ between
    /// agent types. Storing the adapter's own id/value pairs is what lets the
    /// selection be replayed verbatim on reopen. Absent for records written
    /// before this field existed, which restore the adapter's own defaults.
    /// See `openspec/specs/session-setup-persistence/spec.md`.
    #[serde(default)]
    pub session_config:  BTreeMap<String, String>,
    /// Sent as its own turn after the initialization prompt on each fresh
    /// session. See `openspec/specs/prompt-library/spec.md`.
    #[serde(default,
            deserialize_with = "super::prompts::de_startup_prompt",
            skip_serializing_if = "Option::is_none")]
    pub startup_prompt:  Option<StartupPrompt>,
    /// When the agent starts on its own, in a new conversation, so its
    /// startup prompt runs (#404). Stored as text; a value this build cannot
    /// read loads as `Schedule::Unreadable` and never runs. See
    /// `openspec/changes/session-scheduling`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule:        Option<crate::schedule::Schedule>,
    /// What happens to the agent after it has been idle (#404). Absent from
    /// older records, which follow the settings default.
    #[serde(default)]
    pub idle_policy:     crate::schedule::IdlePolicy,
    /// The MCP server this terminal was opened to hand the user its owner's
    /// own MCP flow for (#443). Set only on those terminals. They are
    /// throwaway: one left over from an earlier run is removed when Knot
    /// starts, rather than coming back as a stray shell that re-runs the flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_handover:    Option<String>,
}

impl SavedAgent {
    /// Build a saved agent, substituting the default robot avatar when `avatar`
    /// is `None` or empty. Remaining fields start at their defaults; set them
    /// on the returned value as needed.
    pub fn new(id: Uuid, name: impl Into<String>, avatar: Option<String>,
               folder: impl Into<String>)
               -> Self {
        let avatar = avatar.filter(|a| !a.is_empty())
                           .unwrap_or_else(default_avatar);
        Self { id,
               name: name.into(),
               avatar,
               folder: folder.into(),
               agent_type: default_agent_type(),
               created_by: None,
               is_companion: false,
               shell_command: None,
               persona_id: None,
               session_id: None,
               view_mode: ViewMode::default(),
               acp_session_id: None,
               activation_mode: default_activation_mode(),
               locked: false,
               autopilot_off: false,
               description: String::new(),
               capabilities: Capabilities::new(),
               cost_tier: CostTier::default(),
               session_config: BTreeMap::new(),
               startup_prompt: None,
               schedule: None,
               idle_policy: crate::schedule::IdlePolicy::default(),
               mcp_handover: None }
    }
}

// ---------------------------------------------------------------------------
// BenchAgent
// ---------------------------------------------------------------------------

/// A reusable agent template on the bench.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchAgent {
    pub id:             Uuid,
    pub name:           String,
    #[serde(default = "default_avatar")]
    pub avatar:         String,
    pub folder:         String,
    #[serde(default = "default_agent_type")]
    pub agent_type:     String,
    #[serde(default)]
    pub shell_command:  Option<String>,
    #[serde(default)]
    pub persona_id:     Option<Uuid>,
    /// Registry metadata carried onto the agent this template deploys, so a
    /// saved template records a role and not just a folder.
    #[serde(default)]
    pub description:    String,
    #[serde(default)]
    pub capabilities:   Capabilities,
    #[serde(default)]
    pub cost_tier:      CostTier,
    /// Deployed onto the agent in the form held here: a library reference
    /// stays a reference.
    #[serde(default,
            deserialize_with = "super::prompts::de_startup_prompt",
            skip_serializing_if = "Option::is_none")]
    pub startup_prompt: Option<StartupPrompt>,
}

impl BenchAgent {
    /// Build a bench entry with default avatar/agent-type fallback.
    pub fn new(id: Uuid, name: impl Into<String>, avatar: Option<String>,
               folder: impl Into<String>)
               -> Self {
        let avatar = avatar.filter(|a| !a.is_empty())
                           .unwrap_or_else(default_avatar);
        Self { id,
               name: name.into(),
               avatar,
               folder: folder.into(),
               agent_type: default_agent_type(),
               shell_command: None,
               persona_id: None,
               description: String::new(),
               capabilities: Capabilities::new(),
               cost_tier: CostTier::default(),
               startup_prompt: None }
    }
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

/// A saved group of agents: what the user configured about it, and nothing
/// more. How its window was last arranged is [`WorkspaceUiState`], in its own
/// document - see `openspec/specs/settings-persistence/spec.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id:        Uuid,
    pub name:      String,
    pub color_hex: String,
    #[serde(default)]
    pub agent_ids: Vec<Uuid>,
}

/// What the application recorded about how one workspace's window was last
/// arranged. The user never entered any of it and would not miss it if it
/// were discarded, which is why it is not part of [`Workspace`]: where a
/// window sits is the most frequently written value in the store and the
/// least valuable, and it must not be a reason to rewrite the roster.
///
/// The port stores the layout fields as opaque data; it does not interpret
/// them here.
///
/// Every field carries its own serde default, so an entry written before a
/// field existed loads with that field defaulted rather than failing.
///
/// [`Default`] is written out rather than derived: `layout_mode` and
/// `split_ratio` default to `"single"` and `0.5`, which is what every record
/// written before the split already holds. Deriving would give `""` and
/// `0.0`, so a workspace with no entry would read as a layout that does not
/// exist and a divider at the far edge - a change to what the state *means*,
/// which moving this record is not supposed to make.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspaceUiState {
    #[serde(default = "default_workspace_layout")]
    pub layout_mode:           String,
    pub active_agent_ids:      Vec<Uuid>,
    pub focused_pane_index:    i32,
    #[serde(default = "default_split_ratio")]
    pub split_ratio:           f64,
    pub split_ratio_secondary: Option<f64>,
    pub show_dashboard:        Option<bool>,
    pub is_detached:           Option<bool>,
    /// Where this workspace's window was last seen, so reopening it puts
    /// it back rather than re-centring. `None` until the window has been
    /// opened once.
    pub window_bounds:         Option<SavedWindowBounds>,
    /// Whether this workspace's window was open when the app last quit, so
    /// launch can reopen it (`reopen_workspaces_on_launch`). Written once,
    /// at quit, for every entry; `false` for a record that predates it.
    pub open_at_quit:          bool,
    /// The last scheduled run each agent in this workspace had handled -
    /// started, skipped or missed - as Unix seconds (#404). Kept here, not on
    /// the agent, because Knot recorded it rather than the user setting it;
    /// kept at all so a relaunch inside the grace window cannot start the
    /// same run again.
    pub schedule_runs_handled: BTreeMap<Uuid, i64>,
}

/// A window's position and size in logical pixels, as plain numbers -
/// `knot-core` has no GPUI dependency, so the UI converts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SavedWindowBounds {
    pub x:      f32,
    pub y:      f32,
    pub width:  f32,
    pub height: f32,
}

// ---------------------------------------------------------------------------
// serde defaults
// ---------------------------------------------------------------------------

impl Default for WorkspaceUiState {
    fn default() -> Self {
        Self { layout_mode:           default_workspace_layout(),
               active_agent_ids:      Vec::new(),
               focused_pane_index:    0,
               split_ratio:           default_split_ratio(),
               split_ratio_secondary: None,
               show_dashboard:        None,
               is_detached:           None,
               window_bounds:         None,
               open_at_quit:          false,
               schedule_runs_handled: BTreeMap::new(), }
    }
}

fn default_workspace_layout() -> String {
    WORKSPACE_LAYOUT_DEFAULT.to_string()
}

fn default_split_ratio() -> f64 {
    WORKSPACE_SPLIT_RATIO_DEFAULT
}

pub(super) fn default_avatar() -> String {
    DEFAULT_AVATAR.to_string()
}

pub(super) fn default_agent_type() -> String {
    DEFAULT_AGENT_TYPE.to_string()
}

/// The load default for [`SavedAgent::activation_mode`]. Deliberately not
/// `ActivationMode::default()`: see the field's comment.
fn default_activation_mode() -> ActivationMode {
    ActivationMode::Active
}

fn default_persona_type() -> PersonaType {
    PersonaType::User
}

fn default_persona_state() -> PersonaState {
    PersonaState::Enabled
}

// ---------------------------------------------------------------------------
// SavedPullRequest
// ---------------------------------------------------------------------------

/// A pull request Knot saw in an agent's output.
///
/// The record is evidence of what was seen, not a copy of the pull request:
/// no title, no number, no status. Those are fetched and refreshed, because a
/// merged pull request shown as open after a restart is worse than a blank,
/// and there is no way to know a remembered status is still true.
///
/// Identity is the agent plus the canonical URL: the same pull request seen by
/// two agents is two records, because who opened it is part of what is
/// recorded.
///
/// The Swift reference has no counterpart, so the field names here are chosen
/// rather than inherited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPullRequest {
    /// The canonical URL, as `knot_core::pull_request_url::PullRequestUrl`
    /// renders it - never the decorated form the output carried, so two
    /// spellings of one pull request are one record.
    pub url:          String,
    /// The agent whose output carried the URL.
    pub agent_id:     Uuid,
    /// The workspace that agent belonged to when the URL was seen. Held on
    /// the record rather than looked up through the agent, so a list can be
    /// built for a workspace without walking every agent.
    pub workspace_id: Uuid,
    /// When the URL was first seen, in seconds since the Unix epoch.
    ///
    /// Seconds rather than a formatted timestamp because the only thing the
    /// store does with it is order rows newest-first, and an integer cannot
    /// sort wrong across time zones the way a string can. Formatting it for
    /// a person is the view's business.
    pub first_seen:   i64,
}

impl SavedPullRequest {
    /// A record of `url`, seen now, against `agent_id` in `workspace_id`.
    #[must_use]
    pub fn new(url: impl Into<String>, agent_id: Uuid, workspace_id: Uuid) -> Self {
        Self { url: url.into(),
               agent_id,
               workspace_id,
               first_seen: now_unix() }
    }

    /// Whether this record is the same pull request seen by the same agent -
    /// which is what makes recording idempotent.
    #[must_use]
    pub fn is_same_sighting(&self, url: &str, agent_id: Uuid) -> bool {
        self.agent_id == agent_id && self.url == url
    }
}

/// Seconds since the Unix epoch, or `0` if the clock is set before it.
fn now_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                .map_or(0, |since| {
                                    i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
                                })
}

#[cfg(test)]
mod tests;

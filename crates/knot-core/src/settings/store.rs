//! Durable configuration store: scalar settings plus the serialized
//! collections (saved agents, workspaces, personas, bench templates, library
//! prompts, recent repos, recorded pull requests), with decode-tolerant
//! migration, first-launch source-folder detection, and a bounded recent-repos
//! MRU.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! One [`Settings`] value is one settings surface over nine documents, whose
//! locations [`StorePaths`] derives:
//!
//! | Document | Holds | Directory |
//! |---|---|---|
//! | `preferences.json` | every scalar setting | user preferences |
//! | `agents.json` | saved agents | application data |
//! | `workspaces.json` | saved workspaces | application data |
//! | `workspace-ui-state.json` | per-workspace UI state | application data |
//! | `personas.json` | personas | application data |
//! | `bench.json` | bench templates | application data |
//! | `prompts.json` | library prompts | application data |
//! | `recent-repos.json` | recent repositories | application data |
//! | `pull-requests.json` | recorded pull requests | application data |
//!
//! Three kinds, distinguished by what the values are rather than by which
//! screen edits them: the scalars the user tunes, the collections the user
//! built, and - in `workspace-ui-state.json` - what the application recorded
//! about how its own windows were arranged, which the user never entered.
//! That last one is separate so that moving a window, the most frequent write
//! in the store and the least valuable, does not rewrite the roster.
//!
//! Every mutating helper writes immediately, and writes only the document it
//! changed: [`Settings::persist`] is the whole surface, while
//! [`Settings::persist_preferences`] and the per-collection writers are what
//! the helpers actually call. Each document loads on its own, so one that
//! fails to decode costs only what it held and the app still starts.
//!
//! Two one-way migrations run on load, and compose: an installation still
//! holding the single `settings.json` is migrated off it - see [`legacy`] -
//! and a `workspaces.json` whose records still carry UI state is split - see
//! [`workspace_split`]. The legacy migration's own output is combined, so the
//! split runs on its result as well as on a pre-existing document.
//!
//! Two upgrades run on the preferences document. The terminal font's
//! `"SF Mono"` default is replaced value-for-value, and a document written
//! before the two proportional fonts swapped roles has them exchanged once,
//! gated on the `settingsVersion` marker - see
//! [`preferences::migrate_font_roles`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod bench;
mod binding;
mod documents;
mod legacy;
mod library_locations;
mod paths;
mod personas;
mod preferences;
mod prompts;
mod workspace_split;

pub use library_locations::LibraryLocation;
pub use paths::StorePaths;
pub use prompts::PromptReferences;

use super::prompts::Prompt;
pub use super::records::{
    BenchAgent, Persona, SavedAgent, SavedPullRequest, Workspace, WorkspaceUiState,
};
use super::vocabulary::{AiProvider, AppearanceMode, AutopilotAction};
use crate::consts::{
    MARKDOWN_FONT_SIZE_DEFAULT, MCP_PORT_DEFAULT, RECENT_REPOS_MAX, SETTINGS_VERSION_CURRENT,
    SIDEBAR_WIDTH_DEFAULT, SOURCE_FOLDER_CANDIDATES, TERMINAL_FONT_DEFAULT,
    TERMINAL_FONT_SIZE_DEFAULT, TITLE_FONT_DEFAULT, TITLE_FONT_SIZE_DEFAULT, UI_FONT_DEFAULT,
    UI_FONT_SIZE_DEFAULT, VOICE_ENGINE_DEFAULT, VOICE_PUSH_TO_TALK_KEY_DEFAULT,
};
use crate::error::Result;

/// The whole persisted configuration surface. Load with [`Settings::load`],
/// mutate through the helpers (each persists), or set fields directly and call
/// [`Settings::persist`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Which arrangement the persisted document was written under, so a
    /// load-time migration runs exactly once. See [`SETTINGS_VERSION_CURRENT`]
    /// and [`preferences::migrate_font_roles`].
    ///
    /// Its own serde default, rather than the container's: a document with no
    /// `settingsVersion` key predates the marker and must read as `0`, while a
    /// fresh [`Settings::default`] is already current and must not be
    /// migrated.
    #[serde(default = "preferences::de_legacy_settings_version")]
    pub settings_version:               u32,
    pub appearance_mode:                AppearanceMode,
    pub restore_layout_on_launch:       bool,
    pub restore_conversation_on_launch: bool,
    pub reopen_workspaces_on_launch:    bool,
    pub mcp_server_enabled:             bool,
    pub mcp_server_port:                u16,
    pub source_base_folder:             String,
    #[serde(rename = "sourceBaseFolderInitialized")]
    pub source_folder_detected:         bool,
    pub desktop_notifications_enabled:  bool,
    pub markdown_font_size:             i32,
    pub agent_commands:                 BTreeMap<String, String>,
    pub agent_options:                  BTreeMap<String, String>,
    pub terminal_font_name:             String,
    pub terminal_font_size:             f64,
    pub ui_font_name:                   String,
    pub ui_font_size:                   f64,
    pub title_font_name:                String,
    pub title_font_size:                f64,
    /// The workspace sidebar's width in pixels, shared by every workspace
    /// window. Clamped into [`SIDEBAR_WIDTH_MIN`]..=[`SIDEBAR_WIDTH_MAX`] on
    /// load, so a reader never has to clamp it again. See
    /// `openspec/specs/agent-list-ui/spec.md`.
    pub sidebar_width:                  f64,
    pub autopilot_enabled:              bool,
    pub ai_provider:                    AiProvider,
    pub ai_api_key:                     String,
    pub autopilot_action:               AutopilotAction,
    pub autopilot_custom_prompt:        String,
    pub voice_enabled:                  bool,
    pub voice_engine:                   String,
    pub voice_push_to_talk_key:         i32,
    pub voice_auto_insert:              bool,
    /// Which chord sends a Panel-mode prompt: `false` (default) is Enter to
    /// send / Shift+Enter for a newline; `true` swaps them.
    pub agent_panel_shift_enter_sends:  bool,
    /// Collapse a Panel turn's contiguous tool calls into one summary line
    /// instead of a card per call. Off by default, so existing installs keep
    /// the per-call rendering. See
    /// `openspec/specs/collapsed-tool-call-summary/spec.md`.
    pub agent_panel_compact_tool_calls: bool,
    /// The user's customizations of the navigation shortcuts; empty until
    /// one is changed. See `openspec/specs/keybindings/spec.md`.
    pub keybindings:                    super::KeybindingSettings,
    /// MCP servers the user has said they do not use (#402), by name - a
    /// connector offered by the agent's account but never set up, say. Until
    /// such a server connects, it is listed apart as not connected and left
    /// out of what needs attention. See
    /// `openspec/specs/agent-mcp-status/spec.md`.
    pub dismissed_mcp_servers:          BTreeSet<String>,
    /// Repositories the user has trusted to run the programs their git
    /// configuration names, each with the entries it was trusted with.
    /// Empty by default: Knot runs no git in such a repository until the
    /// user says so. See `openspec/specs/git-operations/spec.md`.
    pub trusted_repositories:           Vec<super::TrustedRepository>,
    /// What happens to an agent that has been idle, for every agent whose
    /// own idle setting follows the default (#404). `None` - off - by
    /// default; an unreadable stored value also loads as off. See
    /// `openspec/changes/session-scheduling`.
    #[serde(deserialize_with = "crate::schedule::lenient_idle_rule")]
    pub idle_default:                   Option<crate::schedule::IdleRule>,

    /// The durable collections, each persisted as its own document rather
    /// than as a key of the preferences document - hence `skip`, which also
    /// makes a stray collection key in a hand-edited preferences document
    /// ignored on read and never written back.
    #[serde(skip)]
    pub saved_agents:      Vec<SavedAgent>,
    #[serde(skip)]
    pub saved_workspaces:  Vec<Workspace>,
    /// How each workspace's window was last arranged, keyed by workspace id.
    ///
    /// Beside `saved_workspaces` rather than inside it: the two have
    /// different lifetimes - a workspace's configuration outlives any window,
    /// and its arrangement is meaningless without one - and only this one is
    /// rewritten by a pointer drag. Written by
    /// [`Settings::persist_workspace_ui`] alone.
    #[serde(skip)]
    pub workspace_ui:      BTreeMap<Uuid, WorkspaceUiState>,
    #[serde(skip)]
    pub personas:          Vec<Persona>,
    #[serde(skip)]
    pub bench_agents:      Vec<BenchAgent>,
    #[serde(skip)]
    pub prompts:           Vec<Prompt>,
    #[serde(skip)]
    pub recent_repos:      Vec<String>,
    #[serde(skip)]
    pub pull_requests:     Vec<SavedPullRequest>,
    #[serde(skip)]
    pub library_locations: Vec<LibraryLocation>,

    #[serde(skip)]
    paths: Option<StorePaths>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { settings_version:               SETTINGS_VERSION_CURRENT,
               appearance_mode:                AppearanceMode::default(),
               restore_layout_on_launch:       true,
               restore_conversation_on_launch: false,
               reopen_workspaces_on_launch:    true,
               mcp_server_enabled:             true,
               mcp_server_port:                MCP_PORT_DEFAULT,
               source_base_folder:             String::new(),
               source_folder_detected:         false,
               desktop_notifications_enabled:  true,
               markdown_font_size:             MARKDOWN_FONT_SIZE_DEFAULT,
               agent_commands:                 BTreeMap::new(),
               agent_options:                  BTreeMap::new(),
               terminal_font_name:             TERMINAL_FONT_DEFAULT.to_string(),
               terminal_font_size:             TERMINAL_FONT_SIZE_DEFAULT,
               ui_font_name:                   UI_FONT_DEFAULT.to_string(),
               ui_font_size:                   UI_FONT_SIZE_DEFAULT,
               title_font_name:                TITLE_FONT_DEFAULT.to_string(),
               title_font_size:                TITLE_FONT_SIZE_DEFAULT,
               sidebar_width:                  SIDEBAR_WIDTH_DEFAULT,
               autopilot_enabled:              false,
               ai_provider:                    AiProvider::default(),
               ai_api_key:                     String::new(),
               autopilot_action:               AutopilotAction::default(),
               autopilot_custom_prompt:        String::new(),
               voice_enabled:                  false,
               voice_engine:                   VOICE_ENGINE_DEFAULT.to_string(),
               voice_push_to_talk_key:         VOICE_PUSH_TO_TALK_KEY_DEFAULT,
               voice_auto_insert:              true,
               agent_panel_shift_enter_sends:  false,
               agent_panel_compact_tool_calls: false,
               keybindings:                    super::KeybindingSettings::default(),
               dismissed_mcp_servers:          BTreeSet::new(),
               trusted_repositories:           Vec::new(),
               idle_default:                   None,
               saved_agents:                   Vec::new(),
               saved_workspaces:               Vec::new(),
               workspace_ui:                   BTreeMap::new(),
               personas:                       Vec::new(),
               bench_agents:                   Vec::new(),
               prompts:                        Vec::new(),
               recent_repos:                   Vec::new(),
               pull_requests:                  Vec::new(),
               library_locations:              Vec::new(),
               paths:                          None, }
    }
}

impl Settings {
    /// Load from the platform's preferences and application-data
    /// directories, migrating a legacy single document first if one is
    /// present. A missing, unreadable or malformed document yields that
    /// document's defaults with `Ok` so the app still starts.
    pub fn load() -> Result<Self> {
        match StorePaths::platform() {
            Some(paths) => Self::load_with(paths),
            None => Ok(Self::default()),
        }
    }

    /// Load every document from one directory (tests, alternate profiles).
    /// Same tolerance rules as [`Settings::load`].
    pub fn load_from_root(dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_with(StorePaths::rooted(dir.as_ref()))
    }

    fn load_with(paths: StorePaths) -> Result<Self> {
        // The legacy migration's own output is a combined workspaces
        // document, so the split has to run on its result too - hence the
        // split inside `read_documents` rather than only on the branch that
        // skipped the legacy migration.
        if let Some(migrated) = legacy::migrate(&paths)? {
            return Ok(migrated);
        }
        Ok(Self::read_documents(paths))
    }

    /// Read each document on its own, so one that cannot be read costs only
    /// what it held.
    fn read_documents(paths: StorePaths) -> Self {
        let mut settings = match documents::read_object(&paths.preferences()) {
            Some(value) => Self::from_preferences(value),
            None => Self::default(),
        };
        settings.saved_agents = documents::read_collection(&paths.agents());
        let workspaces = workspace_split::read(&paths);
        settings.saved_workspaces = workspaces.workspaces;
        settings.workspace_ui = workspaces.ui_state;
        settings.personas = documents::read_collection(&paths.personas());
        settings.bench_agents = documents::read_collection(&paths.bench());
        settings.prompts = documents::read_collection(&paths.prompts());
        settings.recent_repos = documents::read_collection(&paths.recent_repos());
        settings.pull_requests = documents::read_collection(&paths.pull_requests());
        settings.library_locations = documents::read_collection(&paths.library_locations());
        settings.prune_workspace_ui();
        settings.paths = Some(paths);
        if workspaces.needs_write {
            // Best effort: a read-only store still loads, it simply splits
            // again next launch. The values are already correct in memory.
            let _ = settings.persist_workspace_ui();
            let _ = settings.persist_workspaces();
        }
        settings
    }

    /// Drop UI state for a workspace that no longer exists.
    ///
    /// On load rather than on deletion, and in this one place. Deleting a
    /// workspace has several paths - the sidebar, the command centre, an
    /// import that replaces the roster - and a missed one would leak an entry
    /// silently and forever. Load is the single funnel every path's result
    /// goes through, so pruning here cannot be bypassed by a new one.
    fn prune_workspace_ui(&mut self) {
        let live: std::collections::BTreeSet<Uuid> =
            self.saved_workspaces.iter().map(|w| w.id).collect();
        self.workspace_ui.retain(|id, _| live.contains(id));
    }
    /// Write every document.
    ///
    /// The whole surface at once, for a caller that changed more than one
    /// kind of value or does not know which. A caller that does know should
    /// use the writer for what it changed: it then does not rewrite five
    /// documents to record one edit, and - the reason that matters - does not
    /// write back its own stale copy of what another window persisted in the
    /// meantime.
    pub fn persist(&self) -> Result<()> {
        self.persist_preferences()?;
        self.persist_roster()?;
        self.persist_workspace_ui()?;
        self.persist_personas()?;
        self.persist_bench()?;
        self.persist_prompts()?;
        self.persist_recent_repos()?;
        self.persist_pull_requests()?;
        self.persist_library_locations()
    }

    /// Write the preferences document: every scalar setting, and nothing
    /// else.
    pub fn persist_preferences(&self) -> Result<()> {
        documents::write(&self.resolved_paths()?.preferences(),
                         &serde_json::to_string_pretty(self)?)
    }

    /// Write the saved-agents and saved-workspaces documents.
    ///
    /// The two together because that is the unit every caller changes: an
    /// agent belongs to a workspace, so the roster is rebuilt from the agent
    /// store as a pair or not at all.
    pub fn persist_roster(&self) -> Result<()> {
        self.persist_agents()?;
        self.persist_workspaces()
    }

    pub(crate) fn persist_agents(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.agents(), &self.saved_agents)
    }

    pub(crate) fn persist_workspaces(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.workspaces(), &self.saved_workspaces)
    }

    /// Write the per-workspace UI-state document, and nothing else.
    ///
    /// The point of the whole arrangement: a window move, resize, split or
    /// detach persists through here, so it costs one small document rather
    /// than a rewrite of the saved agents and saved workspaces the user
    /// built. See `openspec/specs/settings-persistence/spec.md`.
    pub fn persist_workspace_ui(&self) -> Result<()> {
        documents::write_map(&self.resolved_paths()?.workspace_ui_state(),
                             &self.workspace_ui)
    }

    /// The UI state recorded for `workspace`, or the defaults if none was.
    ///
    /// An absent entry is a workspace whose window has not been arranged yet,
    /// which is exactly the default arrangement - so a reader never has to
    /// distinguish "missing" from "never moved".
    pub fn workspace_ui(&self, workspace: Uuid) -> WorkspaceUiState {
        self.workspace_ui
            .get(&workspace)
            .cloned()
            .unwrap_or_default()
    }

    /// Change `workspace`'s UI state and persist the UI-state document.
    ///
    /// Creates the entry if there is none, so a caller does not have to.
    pub fn update_workspace_ui(&mut self, workspace: Uuid,
                               edit: impl FnOnce(&mut WorkspaceUiState))
                               -> Result<()> {
        edit(self.workspace_ui.entry(workspace).or_default());
        self.persist_workspace_ui()
    }

    pub(crate) fn persist_personas(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.personas(), &self.personas)
    }

    fn persist_recent_repos(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.recent_repos(), &self.recent_repos)
    }

    /// Write the recorded-pull-requests document.
    ///
    /// Skipped while there is nothing to record and no document already
    /// exists, so an installation that has never seen a pull request grows no
    /// file for the feature. Once the document exists it is always written,
    /// including when the last record is removed - otherwise a removal would
    /// not survive a restart, which is the one thing the spec says it must.
    pub fn persist_pull_requests(&self) -> Result<()> {
        let path = self.resolved_paths()?.pull_requests();
        if self.pull_requests.is_empty() && !path.exists() {
            return Ok(());
        }
        documents::write_collection(&path, &self.pull_requests)
    }

    /// On first launch with no source folder set, adopt the first existing
    /// directory among the common source locations and mark detection done so
    /// it never runs again.
    pub fn init_source_folder(&mut self) -> Result<()> {
        if self.source_folder_detected {
            return Ok(());
        }
        let expanded: Vec<PathBuf> = SOURCE_FOLDER_CANDIDATES.iter()
                                                             .map(|c| expand_tilde(c))
                                                             .collect();
        let refs: Vec<&Path> = expanded.iter().map(PathBuf::as_path).collect();
        if let Some(found) = detect_source_base_folder(&refs) {
            self.source_base_folder = found.to_string_lossy().into_owned();
        }
        self.source_folder_detected = true;
        self.persist_preferences()
    }

    /// Move `name` to the front of the recent-repos list, de-duplicating, and
    /// cap the list length.
    pub fn add_recent_repo(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        self.recent_repos.retain(|entry| entry != &name);
        self.recent_repos.insert(0, name);
        self.recent_repos.truncate(RECENT_REPOS_MAX);
        self.persist_recent_repos()
    }
}

/// Return the first candidate that is an existing directory.
pub fn detect_source_base_folder(candidates: &[&Path]) -> Option<PathBuf> {
    candidates.iter()
              .find(|path| path.is_dir())
              .map(|path| path.to_path_buf())
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
       && let Some(base) = BaseDirs::new()
    {
        return base.home_dir().join(rest);
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests;

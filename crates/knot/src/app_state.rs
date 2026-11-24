use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

use knot_messaging::DeliveryEvent;
use parking_lot::Mutex;
use uuid::Uuid;

mod notifications;
mod status;

pub(crate) use notifications::*;
pub(crate) use status::*;

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) struct WorkspaceRow {
    pub(crate) id:       Uuid,
    pub(crate) name:     String,
    pub(crate) selected: bool,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) struct AgentRow {
    pub(crate) id:           Uuid,
    pub(crate) avatar:       String,
    pub(crate) name:         String,
    pub(crate) agent_type:   String,
    pub(crate) folder:       String,
    pub(crate) selected:     bool,
    pub(crate) attached:     bool,
    pub(crate) state:        knot_agents::AgentState,
    pub(crate) unread_count: usize,
}

/// One entry in the sidebar's background context menu - the menu that opens
/// on the agent list's empty space rather than on a row, scoped to the
/// workspace rather than to any one agent.
///
/// A separate enum from [`AgentMenuEntry`] rather than an extension of it:
/// the two menus share no item, and this one disables what does not apply
/// where the row's menu omits it, so nothing is gained by forcing both
/// through one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentListBackgroundEntry {
    Separator,
    NewAgent,
    /// A submenu of the bench's entries, deployed into this workspace.
    NewFromBench,
    RestartAll,
    CloseAll,
    DeactivateAll,
    Broadcast,
}

impl AgentListBackgroundEntry {
    /// The user-visible label, or `None` for a separator. Matches the Swift
    /// reference's strings (`Skwad/Views/Sidebar/SidebarView.swift`), except
    /// Deactivate All, which the reference has no counterpart for.
    pub(crate) fn label(self) -> Option<String> {
        match self {
            Self::Separator => None,
            Self::NewAgent => Some(knot_core::l10n::t("menu.sidebar.new_agent")),
            Self::NewFromBench => Some(knot_core::l10n::t("menu.sidebar.new_from_bench")),
            Self::RestartAll => Some(knot_core::l10n::t("menu.sidebar.restart_all")),
            Self::CloseAll => Some(knot_core::l10n::t("menu.sidebar.close_all")),
            Self::DeactivateAll => Some(knot_core::l10n::t("menu.sidebar.deactivate_all")),
            Self::Broadcast => Some(knot_core::l10n::t("menu.sidebar.broadcast")),
        }
    }
}

/// What the sidebar's background menu needs to know about the workspace it
/// opened over. Counts rather than the agents themselves: enablement is the
/// only decision this menu makes, and it turns on nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SidebarMenuFacts {
    /// Agents in the workspace the sidebar is showing.
    pub(crate) agent_count:   usize,
    /// How many of those are running, i.e. have a session to stop.
    pub(crate) running_count: usize,
    /// Entries on the bench, which New from Bench lists whatever the
    /// workspace holds.
    pub(crate) bench_count:   usize,
}

/// One item of the sidebar's background menu: the entry and whether it
/// applies to this workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SidebarMenuItem {
    pub(crate) entry:   AgentListBackgroundEntry,
    /// Meaningless for [`AgentListBackgroundEntry::Separator`], which the
    /// renderer matches before it ever reads this.
    pub(crate) enabled: bool,
}

/// The sidebar background menu's entries, in order, with its one divider.
///
/// Unlike [`agent_context_menu_entries`], this returns every entry every
/// time and marks the ones that do not apply disabled. The row menu opens on
/// a different row each time and is read top to bottom; this one opens on the
/// same empty space every time and is learned by position, which an item set
/// that changes shape defeats (`agent-list-ui`).
pub(crate) fn sidebar_background_menu_entries(facts: SidebarMenuFacts) -> Vec<SidebarMenuItem> {
    use AgentListBackgroundEntry::*;

    let has_agents = facts.agent_count > 0;
    // Deactivate All needs a *running* agent, not merely an existing one:
    // Restart All and Close All apply to a stopped agent, stopping does not.
    let has_running = facts.running_count > 0;
    [(NewAgent, true),
     (NewFromBench, facts.bench_count > 0),
     (RestartAll, has_agents),
     (CloseAll, has_agents),
     (DeactivateAll, has_running),
     (Separator, false),
     (Broadcast, has_agents)].into_iter()
                             .map(|(entry, enabled)| SidebarMenuItem { entry, enabled })
                             .collect()
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) struct LayoutModel {
    pub(crate) workspace_rows:      Vec<WorkspaceRow>,
    pub(crate) selected_agent_rows: Vec<AgentRow>,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn layout_model(store: &knot_agents::AgentStore, agent_selection: Option<Uuid>,
                           attached_ids: &[Uuid], unread_counts: &BTreeMap<Uuid, usize>)
                           -> LayoutModel {
    let current = store.current_workspace_id();
    let workspace_rows = store.workspaces()
                              .iter()
                              .map(|workspace| WorkspaceRow { id:       workspace.id,
                                                              name:     workspace.name.clone(),
                                                              selected: Some(workspace.id)
                                                                        == current, })
                              .collect::<Vec<_>>();

    let selected_agent_rows =
        store.workspaces()
             .iter()
             .find(|workspace| Some(workspace.id) == current)
             .map(|workspace| {
                 workspace.agent_ids
                          .iter()
                          .filter_map(|id| store.agent(*id))
                          .map(|agent| AgentRow { id:           agent.id,
                                                  avatar:       agent.avatar.clone(),
                                                  name:         agent.name.clone(),
                                                  agent_type:   agent.agent_type.clone(),
                                                  folder:       agent.folder.clone(),
                                                  selected:     Some(agent.id) == agent_selection,
                                                  attached:     attached_ids.contains(&agent.id),
                                                  state:        agent.state,
                                                  unread_count: unread_counts.get(&agent.id)
                                                                             .copied()
                                                                             .unwrap_or(0), })
                          .collect::<Vec<_>>()
             })
             .unwrap_or_default();

    LayoutModel { workspace_rows,
                  selected_agent_rows }
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn command_to_send(input: &str) -> Option<&str> {
    let command = input.trim();
    (!command.is_empty()).then_some(command)
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn stale_session_ids(session_ids: &[Uuid], live_ids: &BTreeSet<Uuid>) -> Vec<Uuid> {
    session_ids.iter()
               .copied()
               .filter(|id| !live_ids.contains(id))
               .collect()
}

/// Builds the agent store from persisted layout when
/// `restore_layout_on_launch` is set, otherwise starts empty. Shared between
/// the GPUI shell and the MCP catalog so both render the same data.
///
/// When `restore_conversation_on_launch` is also set, resolves each restored
/// agent's resume-session id: its own persisted ACP session id, then its
/// persisted terminal session id (either is an exact restore), otherwise the
/// most recent session for its `(folder, agent type)` via the `knot-history`
/// provider registry.
///
/// The ACP id comes first because it is the only one a Panel-mode agent -
/// every coding agent - ever records; reading the terminal id alone left
/// every panel agent to the history guess, which picks the wrong
/// conversation when two agents share a folder.
pub(crate) fn build_agent_store(settings: &knot_core::Settings) -> knot_agents::AgentStore {
    if !settings.restore_layout_on_launch {
        // No recorded pull requests either: every one of them names an agent
        // that is not being restored, and a record with no agent has nothing
        // to show it under - the same rule that drops a removed agent's
        // records.
        return knot_agents::AgentStore::new();
    }

    let mut store = knot_agents::AgentStore::from_saved(&settings.saved_agents,
                                                        settings.saved_workspaces.clone());
    store.set_pull_requests(settings.pull_requests.clone());
    // From its own document, like the pull requests above: how a window was
    // arranged is not derivable from the roster, and `from_saved` deliberately
    // does not invent it.
    store.set_workspace_ui(settings.workspace_ui.clone());
    // A terminal opened to hand the user an agent's MCP flow belongs to the
    // run that opened it (#443). One saved by an earlier run would otherwise
    // come back as a stray shell, and re-run the flow when started.
    let ids: Vec<Uuid> = store.agents().iter().map(|agent| agent.id).collect();
    store.remove_mcp_handovers(&ids);

    if settings.restore_conversation_on_launch {
        let persisted: BTreeMap<Uuid, String> =
            settings.saved_agents
                    .iter()
                    .filter_map(|agent| {
                        agent.acp_session_id
                             .clone()
                             .or_else(|| agent.session_id.clone())
                             .map(|sid| (agent.id, sid))
                    })
                    .collect();
        store.resolve_resume_sessions(&persisted, |folder, agent_type| {
                 let provider = knot_history::provider(agent_type)?;
                 provider.load_sessions(folder)
                         .into_iter()
                         .next()
                         .map(|session| session.id)
             });
    }

    store
}

pub(crate) fn agent_selection_for_workspace(store: &knot_agents::AgentStore, workspace_id: Uuid)
                                            -> Option<Uuid> {
    let workspace = store.workspaces()
                         .iter()
                         .find(|workspace| workspace.id == workspace_id)?;
    // Which agents the layout is showing is arrangement, so it comes from the
    // UI state; which agents belong to the workspace is configuration, and
    // comes from the record.
    let active = store.workspace_ui(workspace_id).active_agent_ids;
    active.iter()
          .chain(workspace.agent_ids.iter())
          .find(|id| store.agent(**id).is_some())
          .copied()
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn initial_agent_selection(store: &knot_agents::AgentStore) -> Option<Uuid> {
    store.current_workspace_id()
         .and_then(|id| agent_selection_for_workspace(store, id))
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
/// The slice of an agent the shell paints. [`agent_status_snapshot`] diffs
/// these so the poller only wakes the UI on visible changes, not on every
/// buffer append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentStatusKey {
    pub(crate) id:            Uuid,
    pub(crate) state:         knot_agents::AgentState,
    pub(crate) status_text:   String,
    pub(crate) is_registered: bool,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeliveryNotice {
    pub(crate) recipient_name: String,
    pub(crate) count:          usize,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn delivery_notice(events: &[DeliveryEvent], agents: &[knot_agents::Agent])
                              -> Option<DeliveryNotice> {
    let event = events.last()?;
    let agent = agents.iter().find(|agent| agent.id == event.agent_id)?;
    let count = events.iter()
                      .filter(|event| event.agent_id == agent.id)
                      .count();
    Some(DeliveryNotice { recipient_name: agent.name.clone(),
                          count })
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn agent_status_snapshot(store: &knot_agents::AgentStore) -> Vec<AgentStatusKey> {
    store.agents()
         .iter()
         .map(|agent| AgentStatusKey { id:            agent.id,
                                       state:         agent.state,
                                       status_text:   agent.status_text.clone(),
                                       is_registered: agent.is_registered, })
         .collect()
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn unread_counts_snapshot(messages: &knot_messaging::MessageStore, agent_ids: &[Uuid])
                                     -> BTreeMap<Uuid, usize> {
    agent_ids.iter()
             .copied()
             .map(|id| (id, messages.unread_count(id)))
             .collect()
}

/// A terminal agent's output tracker reporting a state. Through
/// `set_tracked_state`, so output printed after a usage limit - the
/// wrap-up, or the notice itself - does not lift it (#36).
pub(crate) fn apply_terminal_status(store: &Arc<Mutex<knot_agents::AgentStore>>, agent_id: Uuid,
                                    state: knot_agents::AgentState) {
    store.lock().set_tracked_state(agent_id, state);
}

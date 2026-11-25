//! The window's layout actions applied to the store: choosing an agent,
//! choosing a mode and focusing a pane, each running the pure rules in
//! [`layout`] over the workspace's UI state and writing the result back.
//!
//! The rules need two things the UI state does not hold: the workspace's agent
//! order (the sidebar's, which a companion's row follows its owner's in) and a
//! lookup from an owner to its companions. Both are built here, once, so the
//! window cannot build them differently from one action to the next.
//!
//! Nothing here starts an agent or persists the document. Showing an agent in
//! a pane activates it, and a change is written to disk, but both are the
//! window's to do (`workspace-layouts`, design decision 8); a
//! [`LayoutChange`] hands it what it needs: what was shown, what is now, and
//! so which agents were just placed.

use knot_core::{LayoutMode, WorkspaceUiState};
use uuid::Uuid;

use super::AgentStore;
use super::layout;

/// A workspace's layout before and after one action.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutChange {
    pub before: WorkspaceUiState,
    pub after:  WorkspaceUiState,
}

impl LayoutChange {
    /// The agent in the focused pane afterwards, if any pane is shown.
    pub fn focused(&self) -> Option<Uuid> {
        let index = usize::try_from(self.after.focused_pane_index).ok()?;
        self.after.active_agent_ids.get(index).copied()
    }

    /// The agents in a pane now that were in none before, in pane order.
    pub fn placed(&self) -> Vec<Uuid> {
        self.after
            .active_agent_ids
            .iter()
            .copied()
            .filter(|agent| !self.before.active_agent_ids.contains(agent))
            .collect()
    }

    /// Whether the action changed anything a saved layout holds.
    pub fn changed(&self) -> bool {
        self.before != self.after
    }
}

impl AgentStore {
    /// `owner`'s companions that are in `workspace`, in sidebar order.
    pub fn workspace_companions(&self, workspace: Uuid, owner: Uuid) -> Vec<Uuid> {
        let companions = self.companions(owner);
        self.workspaces
            .iter()
            .find(|candidate| candidate.id == workspace)
            .map(|candidate| {
                candidate.agent_ids
                         .iter()
                         .copied()
                         .filter(|id| companions.contains(id))
                         .collect()
            })
            .unwrap_or_default()
    }

    /// Clicking `agent` in `workspace`'s sidebar (or pressing its number); see
    /// [`layout::select`]. `skip_companions` is ⌘ held. An agent that is not
    /// in the workspace changes nothing.
    pub fn select_in_layout(&mut self, workspace: Uuid, agent: Uuid, skip_companions: bool)
                            -> LayoutChange {
        let before = self.workspace_ui(workspace);
        if !self.workspace_holds(workspace, agent) {
            return unchanged(before);
        }
        let after = layout::select(&before, agent, skip_companions, |owner| {
            self.workspace_companions(workspace, owner)
        });
        self.commit_layout(workspace, before, after)
    }

    /// Choosing `mode` from the Layout menu; see [`layout::set_mode`].
    pub fn set_layout_mode(&mut self, workspace: Uuid, mode: LayoutMode) -> LayoutChange {
        let before = self.workspace_ui(workspace);
        let order = self.workspace_order(workspace);
        let after = layout::set_mode(&before, &order, mode);
        self.commit_layout(workspace, before, after)
    }

    /// Focusing pane `index` of `workspace`'s layout. An index past the panes
    /// shown changes nothing.
    pub fn focus_layout_pane(&mut self, workspace: Uuid, index: usize) -> LayoutChange {
        let before = self.workspace_ui(workspace);
        let Ok(focus) = i32::try_from(index)
        else {
            return unchanged(before);
        };
        if index >= before.active_agent_ids.len() {
            return unchanged(before);
        }
        let after = WorkspaceUiState { focused_pane_index: focus,
                                       ..before.clone() };
        self.commit_layout(workspace, before, after)
    }

    fn workspace_order(&self, workspace: Uuid) -> Vec<Uuid> {
        self.workspaces
            .iter()
            .find(|candidate| candidate.id == workspace)
            .map(|candidate| candidate.agent_ids.clone())
            .unwrap_or_default()
    }

    fn workspace_holds(&self, workspace: Uuid, agent: Uuid) -> bool {
        self.workspaces
            .iter()
            .find(|candidate| candidate.id == workspace)
            .is_some_and(|candidate| candidate.agent_ids.contains(&agent))
    }

    /// Writes `after` when it differs, so an action that changes nothing does
    /// not give an unarranged workspace an entry.
    fn commit_layout(&mut self, workspace: Uuid, before: WorkspaceUiState,
                     after: WorkspaceUiState)
                     -> LayoutChange {
        if after != before {
            self.workspace_ui.insert(workspace, after.clone());
        }
        LayoutChange { before, after }
    }
}

fn unchanged(state: WorkspaceUiState) -> LayoutChange {
    LayoutChange { after:  state.clone(),
                   before: state, }
}

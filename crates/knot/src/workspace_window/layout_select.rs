//! Choosing which agent a window's layout focuses, and the one place that
//! turns the layout's focus into `selected_agent` (`workspace-layouts`,
//! "Selecting an agent from the sidebar", "The focused pane"; design decision
//! 3).
//!
//! `selected_agent` is what the menus, the shortcuts, the side panels and the
//! sidebar highlight read, and it always equals the focused pane's agent. The
//! store holds the layout; this module holds the only writes that carry the
//! layout's focus over to the field, so the two cannot drift apart:
//!
//! - [`WorkspaceWindow::adopt_layout`] after an action the user took, which
//!   also starts what the action put in a pane and saves the layout;
//! - [`WorkspaceWindow::sync_selection_with_layout`] once a frame, for a layout
//!   the store changed without the window (an agent over MCP creating a
//!   companion, a removal), which starts nothing.
//!
//! The rules themselves (R1 to R4, the modes) are `knot_agents::layout`'s and
//! are applied by `AgentStore`; what is here is what the window does with the
//! result.
//!
//! Not here: restoring a layout on open, which must not start agents
//! (`open`), and the dividers (`layout`).

use gpui_kit::Context;
use knot_agents::LayoutChange;
use knot_core::LayoutMode;
use uuid::Uuid;

use super::WorkspaceWindow;

/// What adopting a layout change does about the agents it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Showing {
    /// A user action chose these agents: every one the action placed in a
    /// pane starts, and so does the focused one, as selecting an agent in a
    /// single layout always has ("Showing an agent in a pane starts it").
    Start,
    /// An agent was just created. It starts only as its activation mode
    /// starts it, so a passive one waits to be chosen.
    Created,
}

impl WorkspaceWindow {
    /// The user chose `id`: a sidebar click, the agent-number shortcut, a
    /// card on the dashboard. `skip_companions` is ⌘ held, which skips R1
    /// only. Applies the selection rules to the layout, focuses the pane that
    /// holds `id`, starts what the action put in a pane, and saves the layout.
    ///
    /// An agent that is not in this window's workspace has no pane to show it
    /// in; it is selected and started as it was before layouts, and the
    /// layout is left alone.
    pub(super) fn select_agent(&mut self, id: Uuid, skip_companions: bool, cx: &mut Context<Self>) {
        let change = self.store
                         .lock()
                         .select_in_layout(self.workspace_id, id, skip_companions);
        if change.focused() == Some(id) {
            self.adopt_layout(&change, Showing::Start, cx);
        }
        else {
            self.selected_agent = Some(id);
            self.store.lock().set_activated(id, true);
            self.start_sessions(id, cx);
        }
    }

    /// Shows `id`, an agent that has just been created, where selecting it
    /// would: in the focused pane, or in the pane it already has.
    pub(super) fn select_created_agent(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let change = self.store
                         .lock()
                         .select_in_layout(self.workspace_id, id, false);
        if change.focused() == Some(id) {
            self.adopt_layout(&change, Showing::Created, cx);
        }
        else {
            self.selected_agent = Some(id);
            self.start_sessions(id, cx);
        }
    }

    /// Carries `change` over to the window: `selected_agent` becomes the
    /// focused pane's agent, the agents it names are started per `showing`,
    /// and a changed layout is written to disk.
    ///
    /// The store has already applied the change; this is the window's half,
    /// and the only writer of `selected_agent` for a layout action.
    pub(super) fn adopt_layout(&mut self, change: &LayoutChange, showing: Showing,
                               cx: &mut Context<Self>) {
        let Some(focused) = change.focused()
        else {
            return;
        };
        self.selected_agent = Some(focused);
        let mut shown = change.placed();
        if !shown.contains(&focused) {
            shown.push(focused);
        }
        if showing == Showing::Start {
            let mut store = self.store.lock();
            for id in &shown {
                store.set_activated(*id, true);
            }
        }
        if change.changed() {
            self.persist_workspace_ui(cx);
        }
        for id in shown {
            self.start_sessions(id, cx);
        }
    }

    /// Brings `selected_agent` back to the focused pane's agent when the
    /// layout has several panes and the two disagree. A layout the store
    /// changed under the window - a companion an agent created over MCP, an
    /// agent removed, a shell that exited - moves the focus without going
    /// through [`Self::adopt_layout`].
    ///
    /// Starts nothing and writes nothing: nobody asked for these agents to be
    /// shown, and the store's copy is already what is saved. A single layout
    /// is left to `selected_agent`, as it was before layouts.
    pub(super) fn sync_selection_with_layout(&mut self) {
        let ui = self.store.lock().workspace_ui(self.workspace_id);
        if ui.layout_mode == LayoutMode::Single {
            return;
        }
        let focused =
            usize::try_from(ui.focused_pane_index).ok()
                                                  .and_then(|index| ui.active_agent_ids.get(index))
                                                  .copied();
        if focused.is_some() {
            self.selected_agent = focused;
        }
    }
}

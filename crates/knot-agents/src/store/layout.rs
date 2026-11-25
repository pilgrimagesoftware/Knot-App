//! The rules that decide which agents a workspace window shows, and where
//! (`openspec/changes/workspace-layouts`, design decision 2).
//!
//! Every function here is pure: it reads a workspace's [`WorkspaceUiState`]
//! and, where the rule needs them, its agent order and a companion lookup,
//! and returns the new state. Nothing writes the store and nothing starts an
//! agent. Showing an agent in a pane activates it, but that is the window's
//! job (design decision 8), so these rules stay free of side effects and
//! testable without a window.
//!
//! The panes are `active_agent_ids` in order, the focused one is
//! `focused_pane_index`. The agent order is the workspace's `agent_ids`, which
//! is the sidebar's order: a companion's row follows its owner's.
//!
//! This module owns the layout rules only. Geometry (where a pane sits for a
//! ratio) and the divider belong to the `knot` crate.

use knot_core::consts::{
    WORKSPACE_SPLIT_RATIO_DEFAULT, WORKSPACE_SPLIT_RATIO_MAX, WORKSPACE_SPLIT_RATIO_MIN,
};
use knot_core::{LayoutMode, WorkspaceUiState};
use uuid::Uuid;

/// The most companions a companion layout shows beside their owner: the
/// grid's four panes less the owner's.
const COMPANION_LAYOUT_MAX: usize = 3;

/// How a companion came to exist, which decides what its creation does to a
/// layout that does not show its owner (`workspace-layouts`, "A companion
/// whose owner is not shown").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionOrigin {
    /// The user asked for it: the agent row's New Shell Companion or New
    /// Companion.... The window shows the owner and the companion.
    User,
    /// Any other way, such as an agent over MCP. The layout is left alone and
    /// the companion appears in the sidebar only.
    Other,
}

/// The mode that has exactly `count` panes, for the counts a layout can
/// hold. Used when a mode no longer fits the agents it has.
fn mode_for_count(count: usize) -> LayoutMode {
    match count {
        0 | 1 => LayoutMode::Single,
        2 => LayoutMode::SplitVertical,
        3 => LayoutMode::ThreePane,
        _ => LayoutMode::GridFourPane,
    }
}

/// `mode` if `count` agents fill it, else the largest mode they do fill.
/// Agents beyond a mode's pane count are the caller's to drop.
fn fit_mode(mode: LayoutMode, count: usize) -> LayoutMode {
    if count >= mode.pane_count() {
        mode
    }
    else {
        mode_for_count(count)
    }
}

/// The focused pane, clamped to the panes there are.
fn focus_of(ui: &WorkspaceUiState) -> usize {
    usize::try_from(ui.focused_pane_index).unwrap_or(0)
                                          .min(ui.active_agent_ids.len().saturating_sub(1))
}

/// `ui` with its mode, panes and focus replaced; ratios and the rest of the
/// state are carried over, so a mode keeps its dividers across changes.
fn arranged(ui: &WorkspaceUiState, mode: LayoutMode, panes: Vec<Uuid>, focus: usize)
            -> WorkspaceUiState {
    WorkspaceUiState { layout_mode: mode,
                       active_agent_ids: panes,
                       focused_pane_index: i32::try_from(focus).unwrap_or(0),
                       ..ui.clone() }
}

/// Clicking `agent` in the sidebar: the first of four rules that matches
/// (`workspace-layouts`, "Selecting an agent from the sidebar").
///
/// 1. It has companions, pane 0 is focused and `skip_companions` (⌘ held) is
///    off: show its companion layout.
/// 2. It is in a pane: focus that pane.
/// 3. Pane 0's agent has companions: show `agent` alone.
/// 4. Put it in the focused pane, replacing that pane's agent.
///
/// `companions_of` lists an owner's companions in sidebar order.
pub fn select(ui: &WorkspaceUiState, agent: Uuid, skip_companions: bool,
              companions_of: impl Fn(Uuid) -> Vec<Uuid>)
              -> WorkspaceUiState {
    let panes = &ui.active_agent_ids;
    let focus = focus_of(ui);

    let companions = companions_of(agent);
    if !skip_companions && !companions.is_empty() && focus == 0 {
        let shown = std::iter::once(agent).chain(companions.into_iter().take(COMPANION_LAYOUT_MAX))
                                          .collect::<Vec<_>>();
        return arranged(ui, mode_for_count(shown.len()), shown, 0);
    }
    if let Some(index) = panes.iter().position(|pane| *pane == agent) {
        return arranged(ui, ui.layout_mode, panes.clone(), index);
    }
    if panes.first()
            .is_some_and(|first| !companions_of(*first).is_empty())
    {
        return arranged(ui, LayoutMode::Single, vec![agent], 0);
    }
    let mut next = panes.clone();
    match next.get_mut(focus) {
        Some(slot) => *slot = agent,
        None => next.push(agent),
    }
    let focus = focus.min(next.len() - 1);
    arranged(ui, fit_mode(ui.layout_mode, next.len()), next, focus)
}

/// Choosing `mode` from the Layout menu (`workspace-layouts`, "Changing the
/// layout mode"). Agents already in panes keep their order; those past the new
/// pane count are dropped, and new panes are filled with the workspace's other
/// agents in `order`. Single keeps only the focused pane's agent. Focus stays
/// on its agent when that is still shown, else it goes to pane 0.
///
/// If the workspace has too few agents to fill `mode`, the result is the
/// largest mode they do fill; the menu does not offer such a mode.
pub fn set_mode(ui: &WorkspaceUiState, order: &[Uuid], mode: LayoutMode) -> WorkspaceUiState {
    let count = mode.pane_count();
    let focused = ui.active_agent_ids.get(focus_of(ui)).copied();
    let mut next: Vec<Uuid> = if mode == LayoutMode::Single {
        focused.into_iter().collect()
    }
    else {
        ui.active_agent_ids.iter().copied().take(count).collect()
    };
    for id in order {
        if next.len() >= count {
            break;
        }
        if !next.contains(id) {
            next.push(*id);
        }
    }
    let focus = focused.and_then(|agent| next.iter().position(|pane| *pane == agent))
                       .unwrap_or(0);
    arranged(ui, fit_mode(mode, next.len()), next, focus)
}

/// `companion` was just created for `owner` (`workspace-layouts`, "A new
/// companion opens beside its creator" and "A companion whose owner is not
/// shown").
///
/// With the owner in a pane, the companion is added as the last pane (single
/// becomes a vertical split, a split three-pane, three-pane a grid), or, in a
/// full grid, replaces the last pane that is not the owner's. Its pane is
/// focused. With the owner in no pane, a `User` companion replaces the layout
/// with a vertical split of owner and companion, and an `Other` one leaves it
/// alone.
pub fn add_companion(ui: &WorkspaceUiState, owner: Uuid, companion: Uuid,
                     origin: CompanionOrigin)
                     -> WorkspaceUiState {
    let panes = &ui.active_agent_ids;
    if let Some(index) = panes.iter().position(|pane| *pane == companion) {
        return arranged(ui, ui.layout_mode, panes.clone(), index);
    }
    if !panes.contains(&owner) {
        return match origin {
            CompanionOrigin::User => {
                arranged(ui, LayoutMode::SplitVertical, vec![owner, companion], 1)
            }
            CompanionOrigin::Other => ui.clone(),
        };
    }
    let mut next = panes.clone();
    if next.len() >= LayoutMode::GridFourPane.pane_count() {
        // A full grid. The owner is one pane of at most four distinct ones,
        // so there is always another to replace.
        let Some(slot) = next.iter().rposition(|pane| *pane != owner)
        else {
            return ui.clone();
        };
        next[slot] = companion;
        return arranged(ui, ui.layout_mode, next, slot);
    }
    next.push(companion);
    let focus = next.len() - 1;
    arranged(ui, mode_for_count(next.len()), next, focus)
}

/// `agent` left the workspace, by removal or by a move
/// (`workspace-layouts`, "Removal collapses the layout"). `order` is the
/// workspace's agents after it left.
///
/// Its pane goes, and the mode shrinks to fit the panes that remain: a grid
/// with three becomes three-pane, two become a vertical split, fewer than two
/// single. Focus is clamped to the remaining panes. A single layout whose
/// agent is gone shows the first of `order`. An agent in no pane changes
/// nothing.
pub fn remove(ui: &WorkspaceUiState, order: &[Uuid], agent: Uuid) -> WorkspaceUiState {
    if !ui.active_agent_ids.contains(&agent) {
        return ui.clone();
    }
    let mut next: Vec<Uuid> = ui.active_agent_ids
                                .iter()
                                .copied()
                                .filter(|pane| *pane != agent)
                                .collect();
    if next.is_empty() {
        next.extend(order.iter().copied().find(|id| *id != agent));
    }
    let focus = focus_of(ui).min(next.len().saturating_sub(1));
    arranged(ui, fit_mode(ui.layout_mode, next.len()), next, focus)
}

/// `ui` made consistent with the workspace's agents `order`, as it is when the
/// store loads it (`workspace-layouts`, "The saved layout is restored").
///
/// Agents no longer in the workspace, and repeats, are dropped; panes past the
/// mode's count are dropped; the mode is reduced to fit the agents that
/// remain; a layout showing nothing shows the workspace's first agent; the
/// focused pane and both ratios are clamped. An unknown stored mode has
/// already read as single when it was parsed.
pub fn normalize(ui: &WorkspaceUiState, order: &[Uuid]) -> WorkspaceUiState {
    let mut panes: Vec<Uuid> = Vec::new();
    for id in &ui.active_agent_ids {
        if order.contains(id) && !panes.contains(id) && panes.len() < ui.layout_mode.pane_count() {
            panes.push(*id);
        }
    }
    if panes.is_empty() {
        panes.extend(order.first().copied());
    }
    let mode = fit_mode(ui.layout_mode, panes.len());
    let focus = usize::try_from(ui.focused_pane_index).unwrap_or(0)
                                                      .min(panes.len().saturating_sub(1));
    WorkspaceUiState { split_ratio: clamp_ratio(ui.split_ratio),
                       split_ratio_secondary: ui.split_ratio_secondary.map(clamp_ratio),
                       ..arranged(ui, mode, panes, focus) }
}

/// A divider ratio held to the bounds a drag is held to. One that is not a
/// number, which a hand-edited document can hold, reads as the default.
fn clamp_ratio(ratio: f64) -> f64 {
    if ratio.is_finite() {
        ratio.clamp(WORKSPACE_SPLIT_RATIO_MIN, WORKSPACE_SPLIT_RATIO_MAX)
    }
    else {
        WORKSPACE_SPLIT_RATIO_DEFAULT
    }
}

#[cfg(test)]
mod tests;

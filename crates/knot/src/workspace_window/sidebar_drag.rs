//! Reordering the sidebar's agent list by dragging a row.
//!
//! The pattern is the Workspace Manager's (`workspace_manager/drag.rs`):
//! each row decides from the cursor's half which gap it is offering, the
//! gap - not the row - is what is drawn and what the store is asked to
//! move into, and nothing is drawn where a drop would change nothing. What
//! is different here is that a row can belong to a companion group: the
//! gap a row offers is its *group's* edge, not its own, and a companion row
//! offers its owner's group's edge rather than starting a drag of its own.
//!
//! Not here: what a row looks like (`render/sidebar.rs`,
//! `render/sidebar_compact.rs`), and the store's move itself
//! (`knot_agents::AgentStore::move_agent_to_gap`).

use gpui_kit::App;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::DragMoveEvent;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Point;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use super::WorkspaceWindow;

#[cfg(test)]
mod tests;

/// What a row carries while it is dragged. Only a non-companion row starts
/// one - see `agent-list-ui`'s "Companions and pinned rows are not
/// reordered on their own".
#[derive(Clone)]
pub(crate) struct AgentDrag {
    pub(crate) id:  Uuid,
    /// Its place in the order the drag started from, which is the order
    /// every gap is counted in.
    pub(crate) row: usize,
}

/// Which edge of a row - or, for a companion row, of its owner's group -
/// the cursor is nearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropEdge {
    Top,
    Bottom,
}

/// Where a row sits in the roster: whether it is a companion, which the
/// dragged unit and the drop place both have to resolve through to their
/// group's edges rather than the row's own.
#[derive(Clone, Copy)]
pub(crate) struct AgentRowFlags {
    pub(crate) is_companion: bool,
}

/// The row at or before `row` that owns its group - itself, if `row` is not
/// a companion. Companions always directly follow their owner, so the
/// nearest non-companion row at or above `row` is always it.
fn owner_row(rows: &[AgentRowFlags], row: usize) -> usize {
    (0..=row).rev()
             .find(|&index| !rows[index].is_companion)
             .unwrap_or(row)
}

/// The gap directly after `owner`'s whole group: `owner` plus every
/// companion directly following it.
fn group_end(rows: &[AgentRowFlags], owner: usize) -> usize {
    let mut end = owner + 1;
    while end < rows.len() && rows[end].is_companion {
        end += 1;
    }
    end
}

/// The gap a drop over `edge` of `row` would move the dragged row `dragged`
/// into, or `None` when that gap is on either side of the dragged group and
/// the drop would leave the order as it is.
///
/// `row`'s own group, not `row` itself, is what the edge resolves to: the
/// upper half of any row in a group offers the gap above the whole group,
/// and the lower half offers the gap after it. This is what keeps a drop
/// from ever landing between an owner and its companions, or between two
/// companions of the same owner.
pub(crate) fn drop_gap(rows: &[AgentRowFlags], row: usize, edge: DropEdge, dragged: usize)
                       -> Option<usize> {
    let owner = owner_row(rows, row);
    let end = group_end(rows, owner);
    let gap = match edge {
        DropEdge::Top => owner,
        DropEdge::Bottom => end,
    };
    let dragged_end = group_end(rows, dragged);
    (gap != dragged && gap != dragged_end).then_some(gap)
}

/// The line a row draws for a gap, if the gap is one of its edges. Identical
/// in shape to the Workspace Manager's: every gap but the last is the top
/// edge of the row below it, and only the last row draws a bottom edge, for
/// the gap below it. Group boundaries fall on row indices the same way a
/// plain list's do, so this needs no group awareness of its own.
pub(crate) fn drop_line_edge(row: usize, rows: usize, gap: usize) -> Option<DropEdge> {
    if gap == row {
        Some(DropEdge::Top)
    }
    else if gap == rows && row + 1 == rows {
        Some(DropEdge::Bottom)
    }
    else {
        None
    }
}

/// Where a dragged row would land, and where each row was last painted.
#[derive(Default)]
pub(crate) struct AgentRowDrag {
    pub(crate) target:     Option<DropTarget>,
    /// Each row's bounds from the last frame, in row order - see
    /// `workspace_manager::state::RowDrag::row_bounds`.
    pub(crate) row_bounds: std::rc::Rc<std::cell::RefCell<Vec<Bounds<Pixels>>>>,
    /// This frame's rows, in order - the companion flags `drop_gap` resolves
    /// group edges from. Rebuilt every render; read only during a drag.
    pub(crate) rows:       Vec<AgentRowFlags>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DropTarget {
    pub(crate) row: usize,
    /// Counted in the current order: `n` is above row `n`, `len` below the
    /// last. What [`knot_agents::AgentStore::move_agent_to_gap`] takes.
    pub(crate) gap: usize,
}

impl WorkspaceWindow {
    /// A drag moved somewhere in the sidebar, as heard by row `row`: offer
    /// the gap at the cursor's edge when the cursor is over this row,
    /// withdraw this row's offer when it is not.
    pub(super) fn agent_drag_moved(&mut self, row: usize, event: &DragMoveEvent<AgentDrag>,
                                   cx: &mut Context<Self>) {
        let position = event.event.position;
        let dragged = event.drag(cx).row;
        let rows = self.agent_drag.rows.clone();
        let gap = event.bounds
                       .contains(&position)
                       .then(|| {
                           if position.y < event.bounds.center().y {
                               DropEdge::Top
                           }
                           else {
                               DropEdge::Bottom
                           }
                       })
                       .and_then(|edge| drop_gap(&rows, row, edge, dragged));
        let target = match gap {
            Some(gap) => Some(DropTarget { row, gap }),
            None if self.agent_drag
                        .target
                        .is_some_and(|target| target.row == row) =>
            {
                None
            }
            None => return,
        };
        if self.agent_drag.target != target {
            self.agent_drag.target = target;
            cx.notify();
        }
    }

    /// The drag was released over row `row`: move into the gap it offered.
    pub(super) fn agent_dropped(&mut self, drag: &AgentDrag, row: usize, cx: &mut Context<Self>) {
        let target = self.agent_drag.target.take();
        cx.notify();
        let Some(target) = target.filter(|target| target.row == row)
        else {
            return;
        };
        if self.store
               .lock()
               .move_agent_to_gap(self.workspace_id, drag.id, target.gap)
        {
            self.persist_agents(cx);
        }
    }

    /// The drag was released below every row: move to the end of the list,
    /// the one drop place no row's own bounds cover.
    pub(super) fn agent_dropped_at_end(&mut self, drag: &AgentDrag, cx: &mut Context<Self>) {
        self.agent_drag.target = None;
        cx.notify();
        let end = self.agent_drag.rows.len();
        if self.store
               .lock()
               .move_agent_to_gap(self.workspace_id, drag.id, end)
        {
            self.persist_agents(cx);
        }
    }
}

/// The insertion line, set in the gap outside `edge` of a row that is
/// `relative()`. Centred in the rows' 4px gap (`gap_1`, `render/mod.rs`),
/// which is narrower than the Workspace Manager's - an offset sized for
/// that one's 8px gap would overlap the neighbouring row by a pixel here.
pub(crate) fn drop_line(edge: DropEdge, cx: &App) -> impl IntoElement + use<> {
    let line = div().debug_selector(|| "agent-drop-line".into())
                    .absolute()
                    .left_0()
                    .right_0()
                    .h(px(2.))
                    .rounded_full()
                    .bg(cx.theme().drag_border);
    match edge {
        DropEdge::Top => line.top(px(-3.)),
        DropEdge::Bottom => line.bottom(px(-3.)),
    }
}

/// What follows the cursor during a drag: a copy of the row it was lifted
/// from. The row itself is the drag source - unlike the Workspace Manager's
/// rows, an agent row carries no buttons or text fields that would make a
/// separate handle necessary - so the preview is anchored to the row's own
/// origin rather than a handle's.
pub(crate) struct AgentDragPreview {
    pub(crate) name:   String,
    pub(crate) avatar: String,
    /// The lifted row's bounds relative to the point the drag started at.
    /// `None` if the row was never painted.
    pub(crate) row:    Option<Bounds<Pixels>>,
}

impl AgentDragPreview {
    /// `row` in window coordinates, `press` the point the drag started from.
    pub(crate) fn new(name: String, avatar: String, row: Option<Bounds<Pixels>>,
                      press: Point<Pixels>)
                      -> Self {
        let row = row.map(|row| Bounds { origin: row.origin - press,
                                         size:   row.size, });
        Self { name, avatar, row }
    }
}

impl Render for AgentDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::base::h_flex;

        let card = h_flex().debug_selector(|| "agent-drag-preview".into())
                           .items_center()
                           .gap_3()
                           .p_2()
                           .rounded(cx.theme().radius)
                           .border_1()
                           .border_color(cx.theme().drag_border)
                           .bg(cx.theme().background)
                           .shadow_lg()
                           .opacity(0.9)
                           .text_color(cx.theme().foreground)
                           .child(div().text_2xl().child(self.avatar.clone()))
                           .child(div().font_semibold().child(self.name.clone()));
        // Positioned inside an empty root rather than by margins on the
        // card: gpui lays the drag view out as a root at the drag's
        // starting point, and a root's margins do not move it.
        let card = match self.row {
            Some(row) => card.absolute()
                             .left(row.origin.x)
                             .top(row.origin.y)
                             .w(row.size.width)
                             .h(row.size.height),
            None => card.w(px(240.)),
        };
        div().child(card)
    }
}

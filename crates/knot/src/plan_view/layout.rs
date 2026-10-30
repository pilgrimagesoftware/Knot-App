//! Laying a parsed plan out as a layered graph: columns by depth, rows
//! within a column.
//!
//! Pure geometry, no GPUI, so the arrangement can be tested without a
//! window. The renderer positions cards at these coordinates and paints
//! edges between the anchor points computed here, which is what keeps the
//! two in step.

use crate::plan_view::mermaid::PlanGraph;

/// Card geometry and spacing, in logical pixels.
pub const CARD_WIDTH: f32 = 168.;
pub const CARD_HEIGHT: f32 = 60.;

/// A card's text, in pixels rather than the UI's rem-based sizes (#31).
/// The card is a fixed `CARD_HEIGHT`; text that scaled with the UI font
/// setting outgrew it and was clipped at the bottom. Fixed metrics keep the
/// label's line budget inside the card at every UI font size.
pub const CARD_TEXT_SIZE: f32 = 11.;
pub const CARD_LINE_HEIGHT: f32 = 14.;
/// The padding above and below a card's text.
pub const CARD_PADDING_Y: f32 = 4.;
/// Label lines a card shows before ellipsizing, over the state line or on
/// its own. A longer label is read in full from the card's tooltip.
pub const LABEL_LINES_WITH_STATE: usize = 2;
pub const LABEL_LINES_ALONE: usize = 3;
pub const COLUMN_GAP: f32 = 56.;
pub const ROW_GAP: f32 = 16.;
pub const PADDING: f32 = 16.;

/// Where one task's card sits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card {
    pub node:  usize,
    pub left:  f32,
    pub top:   f32,
    pub depth: usize,
}

impl Card {
    fn right_anchor(&self) -> (f32, f32) {
        (self.left + CARD_WIDTH, self.top + CARD_HEIGHT / 2.)
    }

    fn left_anchor(&self) -> (f32, f32) {
        (self.left, self.top + CARD_HEIGHT / 2.)
    }
}

/// One dependency, as the two points its curve runs between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub from: (f32, f32),
    pub to:   (f32, f32),
}

/// How far an edge's control points reach horizontally, as a fraction of
/// the gap it spans. Enough curve to separate edges that share an anchor,
/// not so much that a short hop loops.
pub const EDGE_CURVE: f32 = 0.5;
/// The arrowhead's length unit: it is 1.6 of these long and 1.6 wide.
pub const ARROWHEAD: f32 = 5.;

/// Where an edge is painted: its curve's start, two control points and end,
/// and the arrowhead's three corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeCurve {
    pub start:    (f32, f32),
    pub control1: (f32, f32),
    pub control2: (f32, f32),
    pub end:      (f32, f32),
    pub head:     [(f32, f32); 3],
}

impl Edge {
    /// This edge as painted on a canvas whose top-left corner is at
    /// `origin` in window coordinates.
    ///
    /// The layout's coordinates are the diagram's own, from its top-left;
    /// a path is painted in the window's. Painting the layout's numbers
    /// as-is put every line at the window's top-left instead of the
    /// diagram's, detached from the cards and still while they scrolled
    /// (#33).
    pub fn curve(&self, origin: (f32, f32)) -> EdgeCurve {
        let at = |(x, y): (f32, f32)| (x + origin.0, y + origin.1);
        let (from_x, from_y) = at(self.from);
        let (to_x, to_y) = at(self.to);
        let reach = (to_x - from_x).abs() * EDGE_CURVE;
        EdgeCurve { start:    (from_x, from_y),
                    control1: (from_x + reach, from_y),
                    control2: (to_x - reach, to_y),
                    end:      (to_x - ARROWHEAD, to_y),
                    head:     [(to_x, to_y),
                               (to_x - ARROWHEAD * 1.6, to_y - ARROWHEAD * 0.8),
                               (to_x - ARROWHEAD * 1.6, to_y + ARROWHEAD * 0.8)], }
    }
}

/// A laid-out plan, and the canvas it needs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    pub cards:  Vec<Card>,
    pub edges:  Vec<Edge>,
    pub width:  f32,
    pub height: f32,
}

/// Depth of each node: one past its deepest dependency, zero for a node
/// with none.
///
/// Iterative to a fixed point rather than recursive, and capped by the node
/// count, so a cyclic document - which the plan emitter cannot produce but
/// a hand-written `view-mermaid` call can - settles instead of recursing
/// forever.
fn depths(graph: &PlanGraph) -> Vec<usize> {
    let mut depths = vec![0usize; graph.nodes.len()];
    for _ in 0..graph.nodes.len() {
        let mut changed = false;
        for (from, to) in &graph.edges {
            let candidate = depths[*from] + 1;
            if candidate > depths[*to] {
                depths[*to] = candidate;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    depths
}

/// Arranges `graph` into columns by depth, preserving declaration order
/// within each column so a plan reads the way it was written.
pub fn layout(graph: &PlanGraph) -> Layout {
    if graph.is_empty() {
        return Layout::default();
    }
    let depths = depths(graph);
    let column_count = depths.iter().max().copied().unwrap_or_default() + 1;
    let mut filled = vec![0usize; column_count];
    let mut cards: Vec<Card> = Vec::with_capacity(graph.nodes.len());

    for (node, depth) in depths.iter().copied().enumerate() {
        let row = filled[depth];
        filled[depth] += 1;
        cards.push(Card { node,
                          left: PADDING + depth as f32 * (CARD_WIDTH + COLUMN_GAP),
                          top: PADDING + row as f32 * (CARD_HEIGHT + ROW_GAP),
                          depth });
    }

    let edges = graph.edges
                     .iter()
                     .map(|(from, to)| Edge { from: cards[*from].right_anchor(),
                                              to:   cards[*to].left_anchor(), })
                     .collect();
    let rows = filled.iter().max().copied().unwrap_or(1);
    Layout { cards,
             edges,
             width: PADDING * 2.
                    + column_count as f32 * CARD_WIDTH
                    + (column_count.saturating_sub(1)) as f32 * COLUMN_GAP,
             height: PADDING * 2.
                     + rows as f32 * CARD_HEIGHT
                     + (rows.saturating_sub(1)) as f32 * ROW_GAP }
}

#[cfg(test)]
mod tests;

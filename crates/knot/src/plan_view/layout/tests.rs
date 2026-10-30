//! Unit tests for [`super`].

use super::*;
use crate::plan_view::mermaid::parse;

fn chain() -> PlanGraph {
    parse("flowchart LR\n    a[\"a\"]\n    b[\"b\"]\n    c[\"c\"]\n    a --> b\n    b --> c\n")
}

#[test]
fn a_chain_lays_out_one_card_per_column() {
    let laid = layout(&chain());

    assert_eq!(laid.cards.iter().map(|c| c.depth).collect::<Vec<_>>(),
               vec![0, 1, 2]);
    assert!(laid.cards[1].left > laid.cards[0].left);
    assert_eq!(laid.cards[0].top, laid.cards[1].top, "one row each");
}

/// Two independent tasks share a column and stack, which is the shape a
/// fan-in reads as.
#[test]
fn independent_tasks_stack_in_the_first_column() {
    let graph = parse("flowchart LR\n    a[\"a\"]\n    b[\"b\"]\n    c[\"c\"]\n    \
                       a --> c\n    b --> c\n");

    let laid = layout(&graph);

    assert_eq!(laid.cards[0].depth, 0);
    assert_eq!(laid.cards[1].depth, 0);
    assert_eq!(laid.cards[2].depth, 1);
    assert_eq!(laid.cards[0].left, laid.cards[1].left);
    assert!(laid.cards[1].top > laid.cards[0].top);
}

/// Depth is the *longest* path, so a task never sits left of something it
/// depends on, even when it also has a shorter route in.
#[test]
fn a_task_sits_past_its_deepest_dependency() {
    let graph = parse("flowchart LR\n    a[\"a\"]\n    b[\"b\"]\n    c[\"c\"]\n    \
                       a --> b\n    b --> c\n    a --> c\n");

    let laid = layout(&graph);

    assert_eq!(laid.cards[2].depth, 2, "not 1, via the shorter a --> c");
}

#[test]
fn edges_run_from_one_card_s_right_edge_to_the_next_card_s_left() {
    let laid = layout(&chain());

    let edge = laid.edges[0];
    assert_eq!(edge.from,
               (laid.cards[0].left + CARD_WIDTH, laid.cards[0].top + CARD_HEIGHT / 2.));
    assert_eq!(edge.to,
               (laid.cards[1].left, laid.cards[1].top + CARD_HEIGHT / 2.));
}

#[test]
fn the_canvas_covers_every_card() {
    let graph = parse("flowchart LR\n    a[\"a\"]\n    b[\"b\"]\n    c[\"c\"]\n    a --> c\n    \
                       b --> c\n");

    let laid = layout(&graph);

    for card in &laid.cards {
        assert!(card.left + CARD_WIDTH + PADDING <= laid.width + 0.01,
                "{card:?}");
        assert!(card.top + CARD_HEIGHT + PADDING <= laid.height + 0.01,
                "{card:?}");
    }
}

/// The plan emitter cannot produce a cycle, but a hand-written
/// `view-mermaid` call can, and the layout must settle rather than recurse.
#[test]
fn a_cyclic_document_settles_instead_of_looping() {
    let graph = parse("flowchart LR\n    a[\"a\"]\n    b[\"b\"]\n    a --> b\n    b --> a\n");

    let laid = layout(&graph);

    assert_eq!(laid.cards.len(), 2);
    assert!(laid.width.is_finite() && laid.height.is_finite());
}

#[test]
fn an_empty_graph_lays_out_to_nothing() {
    let laid = layout(&PlanGraph::default());

    assert!(laid.cards.is_empty());
    assert_eq!(laid.width, 0.);
}

/// #33: an edge is painted where the canvas is, not at the window's
/// top-left - every point moves with the canvas's origin.
#[test]
fn an_edge_is_painted_relative_to_its_canvas() {
    let edge = Edge { from: (10., 20.),
                      to:   (110., 60.), };

    let home = edge.curve((0., 0.));
    let moved = edge.curve((300., 450.));

    let shift = |(x, y): (f32, f32)| (x + 300., y + 450.);
    assert_eq!(moved.start, shift(home.start));
    assert_eq!(moved.control1, shift(home.control1));
    assert_eq!(moved.control2, shift(home.control2));
    assert_eq!(moved.end, shift(home.end));
    assert_eq!(moved.head, home.head.map(shift));
}

/// At the origin the curve is the one painted before the fix: it starts at
/// the source, ends an arrowhead short of the target, and the head's tip is
/// the target.
#[test]
fn an_edge_at_the_origin_runs_from_source_to_target() {
    let edge = Edge { from: (10., 20.),
                      to:   (110., 60.), };
    let curve = edge.curve((0., 0.));
    assert_eq!(curve.start, (10., 20.));
    assert_eq!(curve.head[0], (110., 60.));
    assert_eq!(curve.end, (110. - super::ARROWHEAD, 60.));
    assert_eq!(curve.control1, (10. + 100. * super::EDGE_CURVE, 20.));
}

/// #31: whatever the label, a card's text fits the card - the label's line
/// budget, the state line when there is one, and the padding. The text is in
/// fixed pixels, so this holds at every UI font size.
#[test]
fn a_cards_text_budget_fits_the_card() {
    let lines = |n: usize| n as f32 * super::CARD_LINE_HEIGHT;
    let padding = 2. * super::CARD_PADDING_Y;
    assert!(lines(super::LABEL_LINES_WITH_STATE + 1) + padding <= CARD_HEIGHT,
            "a clamped label and its state line overflow the card");
    assert!(lines(super::LABEL_LINES_ALONE) + padding <= CARD_HEIGHT,
            "a clamped label alone overflows the card");
}

use super::*;

fn standalone() -> AgentRowFlags {
    AgentRowFlags { is_companion: false, }
}

fn companion() -> AgentRowFlags {
    AgentRowFlags { is_companion: true }
}

/// `a`, `b`, `c`, no companions - the same shape as the Workspace Manager's
/// rows, so the gap arithmetic should answer the same way.
fn three_standalone() -> Vec<AgentRowFlags> {
    vec![standalone(), standalone(), standalone()]
}

#[test]
fn a_gap_beside_a_standalone_dragged_row_is_no_drop_at_all() {
    let rows = three_standalone();
    assert_eq!(drop_gap(&rows, 1, DropEdge::Top, 1),
               None,
               "its own top edge");
    assert_eq!(drop_gap(&rows, 1, DropEdge::Bottom, 1),
               None,
               "its own bottom edge");
    assert_eq!(drop_gap(&rows, 0, DropEdge::Bottom, 1),
               None,
               "the edge it shares with the row above");
    assert_eq!(drop_gap(&rows, 2, DropEdge::Top, 1),
               None,
               "the edge it shares with the row below");
    assert_eq!(drop_gap(&rows, 0, DropEdge::Top, 1), Some(0));
    assert_eq!(drop_gap(&rows, 2, DropEdge::Bottom, 1), Some(3));
}

/// `a`, `a`'s companion, `b` - dragging `a` carries its companion, so the
/// group's own span is rows 0..2, not just row 0.
#[test]
fn a_gap_inside_the_dragged_owners_own_group_is_no_drop() {
    let rows = vec![standalone(), companion(), standalone()];
    // Hovering the companion's own row (row 1) resolves to its owner's
    // group, same as hovering the owner directly.
    assert_eq!(drop_gap(&rows, 1, DropEdge::Top, 0),
               None,
               "the owner's own top edge");
    assert_eq!(drop_gap(&rows, 1, DropEdge::Bottom, 0),
               None,
               "the group's own bottom edge");
    assert_eq!(drop_gap(&rows, 2, DropEdge::Bottom, 0), Some(3));
}

/// `a`, `a`'s companion, `b`, `c` - dragging `c` (row 3) must never land
/// between `a` and its companion (gap 1), whichever half of either row it
/// is hovered over.
#[test]
fn a_companion_group_offers_only_its_own_edges() {
    let rows = vec![standalone(), companion(), standalone(), standalone()];
    // Row 0 (the owner): top offers gap 0, bottom offers the group's end,
    // gap 2 - never gap 1, which would split the group.
    assert_eq!(drop_gap(&rows, 0, DropEdge::Top, 3), Some(0));
    assert_eq!(drop_gap(&rows, 0, DropEdge::Bottom, 3), Some(2));
    // Row 1 (the companion): same two gaps, resolved through its owner.
    assert_eq!(drop_gap(&rows, 1, DropEdge::Top, 3), Some(0));
    assert_eq!(drop_gap(&rows, 1, DropEdge::Bottom, 3), Some(2));
}

/// Releasing below the last row is the gap at the end of the list.
#[test]
fn the_end_of_the_list_is_a_drop_place() {
    let rows = three_standalone();
    assert_eq!(drop_gap(&rows, 2, DropEdge::Bottom, 0), Some(3));
}

#[test]
fn each_gap_is_drawn_once_on_the_row_below_it_or_under_the_last() {
    let rows = 3;
    let drawn = |gap| {
        (0..rows).filter_map(|row| drop_line_edge(row, rows, gap).map(|edge| (row, edge)))
                 .collect::<Vec<_>>()
    };
    assert_eq!(drawn(0), vec![(0, DropEdge::Top)]);
    assert_eq!(drawn(2), vec![(2, DropEdge::Top)]);
    assert_eq!(drawn(3), vec![(2, DropEdge::Bottom)]);
}

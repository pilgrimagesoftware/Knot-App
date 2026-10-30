//! The plan pane: a committed task graph drawn as a diagram.
//!
//! Contract: `openspec/specs/task-graph/spec.md`'s "A committed plan is
//! shown to the user".
//!
//! Reads `Agent::mermaid_source`, which until now nothing rendered - the
//! `view-mermaid` tool wrote it and it stopped there. So this also makes
//! that tool do something visible, for every agent and not just an
//! orchestrator's plan.
//!
//! Split three ways on purpose. [`mermaid`] recovers the graph from the
//! text and [`layout`] arranges it, both pure and both tested without a
//! window; this module only turns the arrangement into elements. Layout
//! bugs are the likely ones, and they are the ones a test can catch.

pub mod layout;
pub mod mermaid;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    AnyElement, App, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, PathBuilder,
    StatefulInteractiveElement, Styled, canvas, div, point, px,
};

use crate::plan_view::layout::{
    CARD_HEIGHT, CARD_LINE_HEIGHT, CARD_PADDING_Y, CARD_TEXT_SIZE, CARD_WIDTH, Edge,
    LABEL_LINES_ALONE, LABEL_LINES_WITH_STATE, Layout,
};
use crate::plan_view::mermaid::PlanGraph;

const EDGE_WIDTH: f32 = 1.5;

/// The accent for a task state. Unknown states - a class this build does
/// not know - draw neutral rather than being dropped, so a diagram from a
/// newer emitter still reads.
fn state_color(state: Option<&str>, cx: &App) -> Hsla {
    match state {
        Some("done") => cx.theme().success,
        Some("failed") | Some("blocked") => cx.theme().danger,
        Some("dispatched") => cx.theme().accent,
        Some("ready") => cx.theme().info,
        _ => cx.theme().muted_foreground,
    }
}

/// One task, as a card at the position the layout gave it.
///
/// A card is read, not operated (#31). Its label is cut to what fits, with
/// the whole label and state in a tooltip on hover. It is neither resizable
/// nor movable: the layout is computed from the plan's dependencies, and
/// the next update would put it back. And a click does nothing, so the card
/// keeps the default cursor rather than offering one: the task's agent and
/// its conversation are already a click away in the sidebar.
fn card(index: usize, node: &mermaid::PlanNode, left: f32, top: f32, cx: &App) -> AnyElement {
    let accent = state_color(node.state.as_deref(), cx);
    let label_lines = if node.state.is_some() {
        LABEL_LINES_WITH_STATE
    }
    else {
        LABEL_LINES_ALONE
    };
    let tooltip = match &node.state {
        Some(state) => format!("{}\n{state}", node.label),
        None => node.label.clone(),
    };
    div().id(("plan-card", index))
         .debug_selector(move || format!("plan-card-{index}"))
         .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
         .absolute()
         .left(px(left))
         .top(px(top))
         .w(px(CARD_WIDTH))
         .h(px(CARD_HEIGHT))
         .flex()
         .rounded_md()
         .overflow_hidden()
         .border_1()
         .border_color(cx.theme().border)
         .bg(cx.theme().secondary)
         // The state reads as a stripe down the leading edge rather than
         // as a fill: a filled card competes with its own label, and the
         // stripes line up into a column that can be scanned.
         .child(div().w(px(4.)).h_full().flex_shrink_0().bg(accent))
         .child(div().flex_1()
                     .min_w_0()
                     .px_2()
                     .py(px(CARD_PADDING_Y))
                     .text_size(px(CARD_TEXT_SIZE))
                     .line_height(px(CARD_LINE_HEIGHT))
                     .child(div().debug_selector(move || format!("plan-card-label-{index}"))
                                 .font_weight(FontWeight::MEDIUM)
                                 .text_color(cx.theme().foreground)
                                 .line_clamp(label_lines)
                                 .text_ellipsis()
                                 .child(node.label.clone()))
                     .child(div().text_color(accent)
                                 .child(node.state.clone().unwrap_or_default())))
         .into_any_element()
}

/// Paints every dependency as a curve from one card's right edge to the
/// next card's left edge, with a filled arrowhead at the target.
///
/// Painted at the canvas's own origin: a path is in window coordinates,
/// and the layout's are the diagram's - see [`Edge::curve`].
fn edges(edge_list: Vec<Edge>, color: Hsla) -> AnyElement {
    canvas(move |bounds, _, _| {
               let origin = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
               let at = |(x, y): (f32, f32)| point(px(x), px(y));
               edge_list.iter()
                        .filter_map(|edge| {
                            let curve = edge.curve(origin);
                            let mut line = PathBuilder::stroke(px(EDGE_WIDTH));
                            line.move_to(at(curve.start));
                            line.cubic_bezier_to(at(curve.end),
                                                 at(curve.control1),
                                                 at(curve.control2));

                            let mut head = PathBuilder::fill();
                            head.add_polygon(&curve.head.map(at), true);
                            Some((line.build().ok()?, head.build().ok()?))
                        })
                        .collect::<Vec<_>>()
           },
           move |_, paths, window, _| {
               for (line, head) in paths {
                   window.paint_path(line, color);
                   window.paint_path(head, color);
               }
           }).absolute()
             .size_full()
             .into_any_element()
}

/// The diagram for `source`, or `None` when there is nothing to draw.
///
/// `None` rather than an empty box: the caller decides what an empty plan
/// looks like, and a blank pane where a diagram should be reads as a bug.
pub fn plan_diagram(source: &str, cx: &App) -> Option<AnyElement> {
    let graph: PlanGraph = mermaid::parse(source);
    if graph.is_empty() {
        return None;
    }
    let laid: Layout = layout::layout(&graph);
    let mut canvas_layer =
        div().relative()
             .w(px(laid.width))
             .h(px(laid.height))
             // Edges first, so a card always draws over the line that
             // reaches it rather than being crossed by it.
             .child(edges(laid.edges.clone(), cx.theme().muted_foreground.opacity(0.7)));
    for (index, placed) in laid.cards.iter().enumerate() {
        canvas_layer = canvas_layer.child(card(index,
                                               &graph.nodes[placed.node],
                                               placed.left,
                                               placed.top,
                                               cx));
    }
    Some(canvas_layer.into_any_element())
}

#[cfg(test)]
mod tests;

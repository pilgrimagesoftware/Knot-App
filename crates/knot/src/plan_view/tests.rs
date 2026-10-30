//! The plan pane drawn in a window (#31): a card's label is laid out inside
//! the card.
//!
//! gpui's test text system does not wrap, so a long label is one line here
//! and this cannot show the clamp at work - `layout`'s
//! `a_cards_text_budget_fits_the_card` holds the line budget the clamp
//! enforces. What this does pin is that the card draws its label at the
//! card's fixed text metrics, inside the card, on a real render.

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div,
};

use super::plan_diagram;

/// A label far longer than a card holds, on a task with a state.
const SOURCE: &str = "flowchart LR\n    a[\"Propose open-in-background change (modifier-click / \
                      middle-click on a pod opens it in a background tab instead of \
                      replacing the current view)\"]:::done\n    b[\"next\"]\n    a --> b\n";

struct Plan;

impl Render for Plan {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let diagram: AnyElement = plan_diagram(SOURCE, cx).expect("the plan parses");
        div().size_full().child(diagram)
    }
}

#[gpui_kit::test]
fn a_long_label_stays_inside_its_card(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (_, window) = cx.add_window_view(|_, _| Plan);
    let window: &mut VisualTestContext = window;
    window.run_until_parked();

    let card = window.debug_bounds("plan-card-0")
                     .expect("the card is drawn");
    let label = window.debug_bounds("plan-card-label-0")
                      .expect("the label is drawn");
    assert!(label.bottom() <= card.bottom(),
            "the label runs past its card: {label:?} in {card:?}");
}

//! Which rendered Markdown blocks a selection takes (#491), drawn through
//! Knot's own [`crate::markdown_view`] so the view under test is the one the
//! panel uses.
//!
//! Each test drags across the whole response, the way the user selects. Not
//! select-all: that copies the document's text wholesale, past the per-block
//! walk a drag goes through, so a block left out of selection - as a heading
//! drawn by a claimed custom node was - still reaches a select-all copy.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::Modifiers;
use gpui_kit::MouseButton;
use gpui_kit::Render;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::base::text::TextViewState;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use gpui_kit::point;
use gpui_kit::px;

use crate::markdown_view::{MarkdownStyle, markdown_view_for};

struct SelectionProbe {
    state: Entity<TextViewState>,
}

impl Render for SelectionProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        markdown_view_for(&self.state,
                          "Helvetica".into(),
                          &MarkdownStyle::new(cx.theme(), "Helvetica".into(), px(14.)))
    }
}

/// What a drag from the view's top-left corner to its bottom-right copies
/// from `source`, drawn as a response draws it.
fn drag_across(source: &'static str, cx: &mut TestAppContext) -> String {
    let mut state = None;
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let text = cx.new(|cx| TextViewState::markdown(source, cx));
                             state = Some(text.clone());
                             let view = cx.new(|_| SelectionProbe { state: text });
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("the probe window should open")
                   });
    let state = state.expect("the probe built its state");
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    let bounds = state.read_with(&cx, |state, _| state.bounds());
    let start = bounds.origin + point(px(1.), px(1.));
    let end = bounds.bottom_right() - point(px(1.), px(1.));
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    state.read_with(&cx, |state, _| state.selected_text())
}

/// The reported case: a heading's words, and the paragraph after it, both
/// reach the copy - in that order.
#[gpui_kit::test]
fn a_selection_takes_the_heading_and_the_paragraph_after_it(cx: &mut TestAppContext) {
    let copied = drag_across("## Heading words\n\nThe paragraph after it.\n", cx);
    let heading = copied.find("Heading words")
                        .unwrap_or_else(|| panic!("the heading is selected: {copied:?}"));
    let paragraph = copied.find("The paragraph after it.")
                          .unwrap_or_else(|| panic!("the paragraph is selected: {copied:?}"));
    assert!(heading < paragraph, "in reading order: {copied:?}");
}

/// A selection begun in a paragraph keeps a heading it crosses. This one held
/// even for the old claimed node, which a drag could cross but not start or
/// end on. `a_selection_takes_the_heading_and_the_paragraph_after_it` and
/// `every_heading_level_is_selectable`, which start the drag on a heading,
/// are the ones that caught #491.
#[gpui_kit::test]
fn a_selection_across_a_heading_keeps_its_words(cx: &mut TestAppContext) {
    let copied = drag_across("Words before.\n\n## The middle heading\n\nWords after.\n",
                             cx);
    for words in ["Words before.", "The middle heading", "Words after."] {
        assert!(copied.contains(words), "{words:?} is selected: {copied:?}");
    }
}

/// Every heading level, not only the one in the report.
#[gpui_kit::test]
fn every_heading_level_is_selectable(cx: &mut TestAppContext) {
    let copied = drag_across("# One\n\n## Two\n\n### Three\n\n#### Four\n\n##### Five\n\n###### \
                             Six\n",
                             cx);
    for words in ["One", "Two", "Three", "Four", "Five", "Six"] {
        assert!(copied.contains(words), "{words} is selected: {copied:?}");
    }
}

/// The other block kinds a response draws, so a later custom renderer that
/// takes one of them out of selection fails here too. The drag starts and ends
/// in a paragraph, where a selection can begin whatever sits between.
#[gpui_kit::test]
fn every_other_block_kind_is_selectable(cx: &mut TestAppContext) {
    let copied = drag_across("Opening words.\n\n- a list item\n\n> a quoted line\n\n| head \
                              |\n| --- |\n| a table cell |\n\n```\na code line\n```\n\nA \
                              **bold** closing paragraph.\n",
                             cx);
    for words in ["Opening words.",
                  "a list item",
                  "a quoted line",
                  "a table cell",
                  "a code line",
                  "bold"]
    {
        assert!(copied.contains(words), "{words:?} is selected: {copied:?}");
    }
}

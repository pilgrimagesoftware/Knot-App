//! Marks inside a Markdown header (#491), read through Knot's own
//! [`crate::markdown_view`] in a test window.
//!
//! Pins the `acp-panel-ui` scenario "A header containing a mark": a response
//! containing `## A **bold** word` renders "A bold word" with "bold" kept as a
//! bold run and no asterisks, and inline code inside a header stays code
//! (drawn monospace).
//!
//! # What is observed, and why
//!
//! `gpui-base` exposes no laid-out text runs, so the face is read through the
//! one public path that depends on it: a drag selection over the header. The
//! selected text is the words as drawn, with no markup. The selected source
//! range (`selected_source_range`) is derived from parser positions that the
//! header's inline nodes retain, so it spans the `**` or `` ` `` delimiters
//! only while the header keeps those marks as marks. A header flattened to one
//! face of plain text has no inline nodes to carry positions (and the old
//! claimed heading node was left out of the selection altogether).
//! (`SelectionFormat::Source` is no use here: `TextView` resets the format on
//! every render, and Knot's views leave it at `Plain`.)
//!
//! The drag runs across the whole response, as `markdown_selection` does.

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

struct HeaderProbe {
    state: Entity<TextViewState>,
}

impl Render for HeaderProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        markdown_view_for(&self.state,
                          "Helvetica".into(),
                          &MarkdownStyle::new(cx.theme(), "Helvetica".into(), px(14.)))
    }
}

/// What dragging across `source`, drawn as a response draws it, copies as
/// `(plain text, source slice)`, read from the same selection.
fn drag_across(source: &'static str, cx: &mut TestAppContext) -> (String, String) {
    let mut state = None;
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let text = cx.new(|cx| TextViewState::markdown(source, cx));
                             state = Some(text.clone());
                             let view = cx.new(|_| HeaderProbe { state: text });
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
    state.read_with(&cx, |state, _| {
             let plain = state.selected_text();
             let range =
                 state.selected_source_range()
                      .expect("the header is selectable: a drag over it maps to a source range");
             (plain, source[range].to_owned())
         })
}

/// `## A **bold** word` renders "A bold word", no asterisks, and keeps
/// "bold" as a bold run.
///
/// Plain: the words, without the asterisks. The source range keeps `**bold**`,
/// so "bold" is still a bold mark on the header's inline node.
#[gpui_kit::test]
fn a_header_keeps_bold_as_a_bold_run(cx: &mut TestAppContext) {
    let (plain, source) = drag_across("## A **bold** word\n", cx);
    assert_eq!(plain.trim_end(),
               "A bold word",
               "no asterisks drawn: {plain:?}");
    assert_eq!(source.trim_end(),
               "A **bold** word",
               "bold is a mark, not flattened text: {source:?}");
}

/// `## Run `make` now` renders "Run make now" with `make` still inline code,
/// which `gpui-base` draws monospace.
#[gpui_kit::test]
fn a_header_keeps_inline_code_as_code(cx: &mut TestAppContext) {
    let (plain, source) = drag_across("## Run `make` now\n", cx);
    assert_eq!(plain.trim_end(),
               "Run make now",
               "no backticks drawn: {plain:?}");
    assert_eq!(source.trim_end(),
               "Run `make` now",
               "code is a mark, not flattened text: {source:?}");
}

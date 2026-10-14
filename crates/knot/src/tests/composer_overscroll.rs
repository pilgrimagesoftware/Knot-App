//! The composer scrolls no further than its text (#41).
//!
//! The composer is a `gpui-base` `EditorState`, and an editor counts as a
//! code editor whatever its layout mode, so by default it reserved empty
//! rows below the last line to scroll into. The wheel - or a trackpad
//! gesture brushing past while the user clicks back into the composer -
//! moved the draft up into that space, and since a click only scrolls as
//! far as the caret needs, the draft stayed out of view.
//!
//! Built through the production constructor, so the setting is pinned
//! where it is made.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::Modifiers;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Point;
use gpui_kit::Render;
use gpui_kit::ScrollDelta;
use gpui_kit::ScrollWheelEvent;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::component::input::Position;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;

use crate::workspace_window::panel::prompt::PanelInput;
use crate::workspace_window::panel::prompt::PanelInputState;
use crate::workspace_window::panel::prompt::new_panel_input;
use crate::workspace_window::panel::prompt::panel_input_max_rows;

/// Inside the composer's first line.
const INSIDE: Point<Pixels> = point(px(40.), px(10.));

struct Probe {
    input: Entity<PanelInputState>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
             .child(PanelInput::new(&self.input).w_full())
    }
}

/// A focused composer, built the way the window builds it, holding `text`.
fn composer(cx: &mut TestAppContext, text: &str) -> (VisualTestContext, Entity<PanelInputState>) {
    let mut input = None;
    let window = {
        let input = &mut input;
        cx.update(|cx| {
              gpui_kit::init(cx);
              cx.open_window(WindowOptions::default(), |window, cx| {
                    let state = new_panel_input(false, panel_input_max_rows(false), window, cx);
                    *input = Some(state.clone());
                    let view = cx.new(|_| Probe { input: state });
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("the composer window should open")
          })
    };
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let input = input.expect("the composer was built");
    let text = text.to_string();
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value(text, window, cx);
             state.focus(window, cx);
         });
    cx.run_until_parked();
    (cx, input)
}

/// How far the composer's text is scrolled up, in pixels (zero or less).
fn scroll_top(cx: &mut VisualTestContext, input: &Entity<PanelInputState>) -> f32 {
    cx.update(|_, cx| f32::from(input.read(cx).scroll_offset().y))
}

/// Puts the caret at `position`, which scrolls it into view the way typing
/// there would. Through the editor rather than a keystroke: the
/// go-to-end chord is `cmd-down` on macOS and `ctrl-end` elsewhere, and a
/// macOS chord on Linux CI moves nothing.
fn move_caret(cx: &mut VisualTestContext, input: &Entity<PanelInputState>, position: Position) {
    input.update_in(cx, |state, window, cx| {
             state.set_cursor_position(position, window, cx)
         });
    cx.run_until_parked();
}

/// Turns the wheel over the composer, content-upwards, by `distance`.
fn wheel_down(cx: &mut VisualTestContext, distance: f32) {
    cx.simulate_event(ScrollWheelEvent { position: INSIDE,
                                         delta: ScrollDelta::Pixels(point(px(0.),
                                                                          px(-distance))),
                                         ..Default::default() });
    cx.run_until_parked();
}

/// Focus elsewhere, then a click back into the composer.
fn click_away_and_back(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.blur(cx));
    cx.run_until_parked();
    cx.simulate_click(INSIDE, Modifiers::none());
    cx.run_until_parked();
}

/// The reported defect: a draft the composer shows in full had nowhere to
/// scroll, yet a wheel turn moved all of it out of view, and clicking back
/// in did not bring it back.
#[gpui_kit::test]
fn a_draft_that_fits_cannot_be_scrolled_out_of_view(cx: &mut TestAppContext) {
    let (mut cx, input) = composer(cx, "one\ntwo\nthree");

    for distance in [30., 30., 200., 400.] {
        wheel_down(&mut cx, distance);
        assert_eq!(scroll_top(&mut cx, &input),
                   0.,
                   "the draft fits the composer, so a {distance}px wheel turn has nothing to \
                    scroll");
    }
    click_away_and_back(&mut cx);
    assert_eq!(scroll_top(&mut cx, &input), 0.);
}

/// A draft taller than the composer scrolls - and stops with its last line
/// at the bottom edge, exactly where typing at the end leaves it, rather
/// than past it.
#[gpui_kit::test]
fn a_long_draft_scrolls_no_further_than_its_last_line(cx: &mut TestAppContext) {
    let text = (0..12).map(|n| format!("line {n}"))
                      .collect::<Vec<_>>()
                      .join("\n");
    let (mut cx, input) = composer(cx, &text);
    move_caret(&mut cx, &input, Position::new(11, 7));
    let at_end = scroll_top(&mut cx, &input);
    assert!(at_end < 0.,
            "twelve lines overflow the collapsed composer, so the end is scrolled");

    move_caret(&mut cx, &input, Position::new(0, 0));
    wheel_down(&mut cx, 2_000.);
    assert_eq!(scroll_top(&mut cx, &input),
               at_end,
               "the wheel should stop where the last line meets the bottom edge");

    click_away_and_back(&mut cx);
    assert!(scroll_top(&mut cx, &input) >= at_end,
            "clicking back in must not leave the draft scrolled past its end");
}

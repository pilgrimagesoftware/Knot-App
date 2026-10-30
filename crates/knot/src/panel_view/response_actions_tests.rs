//! A response's action bar on hover (#313): an earlier response shows its
//! copy, reply and scroll controls only while it is hovered, with the bar's
//! row reserved either way; the latest response keeps its bar in view.
//!
//! Clicking is the observable: a hidden bar paints no buttons and so takes
//! no clicks, which is what the user meets, and a copy lands on the
//! clipboard where a test can read it.

use gpui_kit::AppContext;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ListAlignment;
use gpui_kit::ListState;
use gpui_kit::Modifiers;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;

use super::LIST_OVERDRAW;
use super::callbacks::PanelCallbacks;
use super::message::Message;
use super::message::render_message;
use super::message_tests::panel_style;
use crate::panel_state::PanelMessage;
use crate::panel_state::PanelState;

const RESPONSE: &str = "[the docs](https://example.com/docs) hold the answer";
const BAR: &str = "panel-response-actions-0";

/// A view whose body is one finished response, drawn as the latest message
/// or as an earlier one.
struct ResponseProbe {
    state:   PanelState,
    list:    ListState,
    is_last: bool,
}

impl Render for ResponseProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let style = panel_style();
        let message = self.state.messages[0].clone();
        div().size_full()
             .child(render_message(Message { state:    &self.state,
                                             index:    0,
                                             is_last:  self.is_last,
                                             style:    &style,
                                             list:     &self.list,
                                             cmd_held: false,
                                             reply:    None, },
                                   &message,
                                   &PanelCallbacks::new(|_| {}, || {}, |_| {}, |_| {}, || {})))
    }
}

fn response_window(is_last: bool, cx: &mut TestAppContext) -> VisualTestContext {
    let mut state = PanelState::new();
    state.messages
         .push(PanelMessage::Assistant(RESPONSE.to_owned()));
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       crate::link_tooltip::install(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let list = ListState::new(1, ListAlignment::Top, px(LIST_OVERDRAW));
                             let view = cx.new(|_| ResponseProbe { state,
                                                                   list,
                                                                   is_last });
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("the probe window should open")
                   });
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx
}

fn bar_bounds(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.debug_bounds(BAR).expect("the action bar is laid out")
}

/// Rests the pointer at `position` and lets the hover repaint.
fn point_at(cx: &mut VisualTestContext, position: gpui_kit::Point<Pixels>) {
    cx.simulate_mouse_move(position, None, Modifiers::none());
    cx.run_until_parked();
}

/// Clicks the bar's first button, Copy, and returns what the clipboard
/// then holds.
fn click_copy(cx: &mut VisualTestContext) -> Option<String> {
    let bar = bar_bounds(cx);
    cx.simulate_click(point(bar.origin.x + px(8.), bar.center().y),
                      Modifiers::none());
    cx.run_until_parked();
    cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
}

/// Far below the one response, on the empty body.
fn away(cx: &mut VisualTestContext) -> gpui_kit::Point<Pixels> {
    let bar = bar_bounds(cx);
    point(bar.origin.x + px(8.), bar.bottom() + px(200.))
}

#[gpui_kit::test]
fn an_earlier_responses_bar_takes_no_click_until_hovered(cx: &mut TestAppContext) {
    let mut cx = response_window(false, cx);
    let away = away(&mut cx);
    point_at(&mut cx, away);

    assert_eq!(click_copy(&mut cx), None, "a hidden bar is not clickable");
}

#[gpui_kit::test]
fn hovering_an_earlier_response_reveals_its_bar(cx: &mut TestAppContext) {
    let mut cx = response_window(false, cx);
    let text = point(px(12.), bar_bounds(&mut cx).origin.y - px(4.));
    point_at(&mut cx, text);

    assert_eq!(click_copy(&mut cx).as_deref(), Some(RESPONSE));
}

/// Revealing the bar moves nothing: its row is reserved while hidden.
#[gpui_kit::test]
fn revealing_the_bar_moves_nothing(cx: &mut TestAppContext) {
    let mut cx = response_window(false, cx);
    let away = away(&mut cx);
    point_at(&mut cx, away);
    let hidden = bar_bounds(&mut cx);

    point_at(&mut cx, hidden.center());
    assert_eq!(bar_bounds(&mut cx), hidden);
}

/// The latest response keeps its controls without a hover.
#[gpui_kit::test]
fn the_latest_responses_bar_needs_no_hover(cx: &mut TestAppContext) {
    let mut cx = response_window(true, cx);
    let away = away(&mut cx);
    point_at(&mut cx, away);

    assert_eq!(click_copy(&mut cx).as_deref(), Some(RESPONSE));
}

/// A link inside an earlier response shows its URL (#326) and the same
/// pointer still reveals the response's bar (#313): neither hover takes the
/// other's place, since the URL label takes no input.
#[gpui_kit::test]
fn a_link_in_a_response_shows_its_url_and_the_bar_still_appears(cx: &mut TestAppContext) {
    let mut cx = response_window(false, cx);
    point_at(&mut cx, point(px(10.), px(10.)));

    assert!(cx.debug_bounds("link-url-tooltip").is_some(),
            "the link's URL is shown");
    assert_eq!(click_copy(&mut cx).as_deref(), Some(RESPONSE));
}

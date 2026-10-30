//! A link's URL beside the pointer (#326), through the real path: a
//! Markdown `TextView` in a window whose `Root` carries the plugin, the
//! pointer moved onto the link and away again.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::Modifiers;
use gpui_kit::ParentElement;
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

/// One Markdown paragraph that opens with a link, the way a response draws
/// one.
///
/// Or nothing, once `shown` is cleared - as when the agent is switched.
struct LinkProbe {
    shown: bool,
}

impl Render for LinkProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().w(px(400.)).children(self.shown.then(|| {
            crate::markdown_view::markdown_view(
                "link-probe",
                "[the docs](https://example.com/docs) and then some plain words",
                "Helvetica".into(),
                "Helvetica".into(),
                px(14.),
            )
        }))
    }
}

fn window(cx: &mut TestAppContext) -> VisualTestContext {
    window_and_probe(cx).0
}

fn window_and_probe(cx: &mut TestAppContext) -> (VisualTestContext, gpui_kit::Entity<LinkProbe>) {
    let mut probe = None;
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       super::install(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| LinkProbe { shown: true });
                             probe = Some(view.clone());
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("the probe window should open")
                   });
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    (cx, probe.expect("the probe view was built"))
}

fn point_at(cx: &mut VisualTestContext, x: f32, y: f32) {
    cx.simulate_mouse_move(point(px(x), px(y)), None, Modifiers::none());
    cx.run_until_parked();
}

fn tooltip_shown(cx: &mut VisualTestContext) -> bool {
    cx.debug_bounds("link-url-tooltip").is_some()
}

#[gpui_kit::test]
fn resting_on_a_link_shows_its_url_and_leaving_hides_it(cx: &mut TestAppContext) {
    let mut cx = window(cx);
    // The test pointer starts at the window's corner, which is on the link.
    point_at(&mut cx, 10., 300.);
    assert!(!tooltip_shown(&mut cx),
            "nothing is shown away from the link");

    point_at(&mut cx, 10., 10.);
    assert!(tooltip_shown(&mut cx), "the link's URL is shown");
    let url = cx.update(|window, cx| gpui_kit::base::text::hovered_link(window, cx))
                .map(|link| link.url.to_string());
    assert_eq!(url.as_deref(), Some("https://example.com/docs"));

    point_at(&mut cx, 10., 300.);
    assert!(!tooltip_shown(&mut cx), "leaving the link hides it");
}

/// Plain words beside a link show nothing: the label is for links only.
#[gpui_kit::test]
fn plain_text_shows_no_url(cx: &mut TestAppContext) {
    let mut cx = window(cx);
    point_at(&mut cx, 10., 10.);
    assert!(tooltip_shown(&mut cx));

    point_at(&mut cx, 250., 10.);
    assert!(!tooltip_shown(&mut cx));
}

/// The label takes no input, so the link under it stays clickable.
#[gpui_kit::test]
fn the_link_stays_clickable_under_its_label(cx: &mut TestAppContext) {
    let mut cx = window(cx);
    point_at(&mut cx, 10., 10.);
    assert!(tooltip_shown(&mut cx));

    cx.simulate_click(point(px(10.), px(10.)), Modifiers::none());
    assert_eq!(cx.opened_url().as_deref(), Some("https://example.com/docs"));
}

/// The text under a still pointer goes away - the agent switched by
/// keyboard, the row scrolled out of the list - and its URL goes with it,
/// rather than staying over whatever is drawn there now.
#[gpui_kit::test]
fn the_url_goes_when_its_link_is_no_longer_drawn(cx: &mut TestAppContext) {
    let (mut cx, probe) = window_and_probe(cx);
    point_at(&mut cx, 10., 10.);
    assert!(tooltip_shown(&mut cx));

    probe.update(&mut cx, |probe, cx| {
             probe.shown = false;
             cx.notify();
         });
    cx.run_until_parked();

    assert!(!tooltip_shown(&mut cx));
}

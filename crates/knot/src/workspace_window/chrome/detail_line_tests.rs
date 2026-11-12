//! Where a detail line's icon sits against its text (#403; `agent-list-ui`,
//! "Every detail line on an agent row is labelled by an icon"): centred on
//! the line's first text line, at both line sizes, and still on the first
//! line when the text wraps.

use gpui_kit::AppContext;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::assets::IconName;
use gpui_kit::base::v_flex;
use gpui_kit::component::Root;
use gpui_kit::px;

use super::DetailLineSize;
use super::detail_line;

const STATUS: &str = "Working";
const PERSONA: &str = "Rust";
/// Long enough to wrap at [`NARROW`].
const WRAPPING: &str = "DevOps Troubleshooter Extraordinaire";
const NARROW: f32 = 90.;

/// A body of detail lines: one of each size on a wide row, and a persona
/// that wraps on a narrow one.
struct DetailLines;

impl Render for DetailLines {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let line = |icon, text: &str, size| {
            detail_line(icon, text.to_owned(), size, "Helvetica".into(), px(13.), cx)
        };
        v_flex().size_full()
                .child(v_flex().w(px(400.))
                               .child(line(IconName::Activity, STATUS, DetailLineSize::Body))
                               .child(line(IconName::User, PERSONA, DetailLineSize::Small)))
                .child(v_flex().w(px(NARROW))
                               .child(line(IconName::User, WRAPPING, DetailLineSize::Small)))
    }
}

struct Measured {
    icon: Bounds<Pixels>,
    text: Bounds<Pixels>,
}

/// The `debug_selector`s `detail_line` gives a line's icon and text, which
/// `debug_bounds` wants as `'static` strings.
fn selectors(text: &str) -> (&'static str, &'static str) {
    match text {
        STATUS => ("detail-line-icon-Working", "detail-line-text-Working"),
        PERSONA => ("detail-line-icon-Rust", "detail-line-text-Rust"),
        WRAPPING => ("detail-line-icon-DevOps Troubleshooter Extraordinaire",
                     "detail-line-text-DevOps Troubleshooter Extraordinaire"),
        other => panic!("no selectors for {other:?}"),
    }
}

fn measure(cx: &mut VisualTestContext, text: &str) -> Measured {
    let (icon, line) = selectors(text);
    Measured { icon: cx.debug_bounds(icon)
                       .unwrap_or_else(|| panic!("the icon for {text:?} is laid out")),
               text: cx.debug_bounds(line)
                       .unwrap_or_else(|| panic!("the text {text:?} is laid out")), }
}

fn draw(cx: &mut TestAppContext) -> VisualTestContext {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| DetailLines);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("the probe window should open")
                   });
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx
}

fn centre_y(bounds: Bounds<Pixels>) -> f32 {
    f32::from(bounds.origin.y) + f32::from(bounds.size.height) / 2.
}

/// Half a pixel: layout rounds to device pixels.
fn assert_near(actual: f32, expected: f32, what: &str) {
    assert!((actual - expected).abs() <= 0.5,
            "{what}: {actual} is not within half a pixel of {expected}");
}

#[gpui_kit::test]
fn an_icon_is_centred_on_its_single_line(cx: &mut TestAppContext) {
    let mut cx = draw(cx);
    for text in [STATUS, PERSONA] {
        let line = measure(&mut cx, text);
        assert!(line.icon.size.height < line.text.size.height,
                "{text:?}: the icon is shorter than its line, so top and centre differ");
        assert_near(centre_y(line.icon), centre_y(line.text), text);
    }
}

#[gpui_kit::test]
fn a_wrapped_line_keeps_its_icon_centred_on_its_first_line(cx: &mut TestAppContext) {
    let mut cx = draw(cx);
    let one_line = f32::from(measure(&mut cx, PERSONA).text.size.height);
    let wrapped = measure(&mut cx, WRAPPING);

    assert!(f32::from(wrapped.text.size.height) >= one_line * 1.5,
            "{WRAPPING:?} wraps at {NARROW}px");
    let first_line_centre = f32::from(wrapped.text.origin.y) + one_line / 2.;
    assert_near(centre_y(wrapped.icon),
                first_line_centre,
                "the wrapped line's icon");
}

//! The activating click (#309): absorbed by every window it does not belong
//! to a control in, except a drawn title bar's drag strip.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::Root;
use gpui_kit::{
    AppContext, Context, InteractiveElement, IntoElement, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, ParentElement, Point, Render, StatefulInteractiveElement, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};

use super::{absorbs, install};

fn press(position: Point<gpui_kit::Pixels>, first_mouse: bool) -> MouseDownEvent {
    MouseDownEvent { button: MouseButton::Left,
                     position,
                     modifiers: Modifiers::default(),
                     click_count: 1,
                     first_mouse }
}

#[test]
fn only_the_activating_left_click_below_the_drag_strip_is_absorbed() {
    let strip = px(34.);
    assert!(absorbs(&press(point(px(50.), px(100.)), true), strip));
    assert!(!absorbs(&press(point(px(50.), px(100.)), false), strip),
            "a click into a focused window is the window's to handle");
    assert!(!absorbs(&press(point(px(50.), px(10.)), true), strip),
            "the drawn title bar still drags an inactive window");
    let right = MouseDownEvent { button: MouseButton::Right,
                                 ..press(point(px(50.), px(100.)), true) };
    assert!(!absorbs(&right, strip));
}

/// A button that counts its clicks, filling a window with no title bar.
struct Clicks(Rc<Cell<usize>>);

impl Render for Clicks {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let count = Rc::clone(&self.0);
        div().size_full().child(div().id("target")
                                     .size(px(200.))
                                     .on_click(move |_, _, _| count.set(count.get() + 1)))
    }
}

fn click(window: &mut VisualTestContext, first_mouse: bool) {
    let at = point(px(50.), px(50.));
    window.simulate_event(press(at, first_mouse));
    window.simulate_event(MouseUpEvent { button:      MouseButton::Left,
                                         position:    at,
                                         modifiers:   Modifiers::default(),
                                         click_count: 1, });
    window.run_until_parked();
}

/// End to end through a real `Root`: the click that activates the window
/// reaches no control; the next click does.
#[gpui_kit::test]
fn a_window_keeps_its_activating_click(cx: &mut TestAppContext) {
    let clicks = Rc::new(Cell::new(0));
    cx.update(|cx| {
          gpui_kit::init(cx);
          install(cx);
      });
    let counted = Rc::clone(&clicks);
    let handle = cx.add_window(move |window, cx| {
                       let view = cx.new(|_| Clicks(counted));
                       Root::new(view, window, cx)
                   });
    let mut window = VisualTestContext::from_window(handle.into(), cx);
    window.run_until_parked();

    click(&mut window, true);
    assert_eq!(clicks.get(), 0, "the activating click pressed the button");

    click(&mut window, false);
    assert_eq!(clicks.get(),
               1,
               "a click into the focused window is handled");
}

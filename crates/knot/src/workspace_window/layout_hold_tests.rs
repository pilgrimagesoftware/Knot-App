//! What a divider drag does to the panes' contents: a terminal is held at
//! its pre-drag size until the drag ends and so is resized once, on release,
//! while a conversation panel follows the divider live (`workspace-layouts`,
//! "Dividers"; design decision 6).
//!
//! The terminals are real `/bin/cat` PTYs handed to the window the way a
//! started session hands its own, so the views fit and resize as they do in
//! the product; a grid size is what the PTY was last told.

use gpui_kit::Modifiers;
use gpui_kit::MouseButton;
use gpui_kit::TestAppContext;
use gpui_kit::point;
use gpui_kit::px;
use gpui_terminal::Error;
use gpui_terminal::GridSize;
use gpui_terminal::PtyCommand;
use gpui_terminal::PtyTransport;
use gpui_terminal::TerminalBuilder;
use knot_core::LayoutMode;

use super::layout_test_support::Fixture;
use super::layout_test_support::Spec;
use super::layout_test_support::open_as;

const SPLIT: Spec<'static> = Spec { agents:     &["A", "B"],
                                    companions: &[],
                                    mode:       LayoutMode::SplitVertical,
                                    panes:      &["A", "B"],
                                    focus:      0, };

/// Gives `name` a live `/bin/cat` terminal, as starting its session would.
fn attach_terminal(f: &mut Fixture, name: &str) {
    let id = f.id(name);
    f.store.lock().set_activated(id, true);
    let terminal = TerminalBuilder::new().connect(|sink| {
                                             PtyTransport::spawn(&PtyCommand::new("/bin/cat"), sink)
                                         })
                                         .map_err(|error: Error| error)
                                         .expect("a cat process on a pty");
    f.view.update_in(&mut f.window, |view, _, cx| {
              view.open_terminal_pane(id, terminal, cx);
              cx.notify();
          });
}

fn grid(f: &mut Fixture, name: &str) -> GridSize {
    let id = f.id(name);
    f.view.read_with(&f.window, |view, cx| {
              view.terminal_panes[&id].view.read(cx).terminal().size()
          })
}

fn settle(f: &mut Fixture) {
    f.window.run_until_parked();
    f.window.refresh().expect("the window is open");
    f.window.run_until_parked();
}

#[gpui_kit::test]
fn terminals_keep_their_grid_during_a_drag_and_resize_once_on_release(cx: &mut TestAppContext) {
    let mut f = open_as(&SPLIT, knot_core::agent_type::SHELL, cx);
    attach_terminal(&mut f, "A");
    attach_terminal(&mut f, "B");
    settle(&mut f);
    let (a0, b0) = (grid(&mut f, "A"), grid(&mut f, "B"));

    let grab = f.window
                .debug_bounds("layout-divider-primary")
                .expect("a split draws its divider")
                .center();
    f.window
     .simulate_mouse_down(grab, MouseButton::Left, Modifiers::none());
    for step in 1..=4 {
        f.window
         .simulate_mouse_move(point(grab.x + px(25. * step as f32), grab.y),
                              MouseButton::Left,
                              Modifiers::none());
        settle(&mut f);
        assert_eq!(grid(&mut f, "A"), a0, "A's grid held at step {step}");
        assert_eq!(grid(&mut f, "B"), b0, "B's grid held at step {step}");
    }
    let (live, _) = {
        let ui = f.ui();
        (ui.split_ratio, ui.split_ratio_secondary)
    };
    assert!(live > 0.5, "the divider did move, ratio {live}");

    f.window.simulate_mouse_up(point(grab.x + px(100.), grab.y),
                               MouseButton::Left,
                               Modifiers::none());
    settle(&mut f);
    let (a1, b1) = (grid(&mut f, "A"), grid(&mut f, "B"));
    assert_ne!(a1.columns, a0.columns, "A widened on release");
    assert_ne!(b1.columns, b0.columns, "B narrowed on release");

    // Once: further frames leave the grids where the release put them.
    f.window
     .simulate_mouse_move(point(grab.x + px(300.), grab.y), None, Modifiers::none());
    settle(&mut f);
    assert_eq!((grid(&mut f, "A"), grid(&mut f, "B")), (a1, b1));
}

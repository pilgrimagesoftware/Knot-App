//! What follows the focused pane (`workspace-layouts`, "The focused pane" and
//! "The sidebar highlights the focused agent's row"): the side panels, which
//! wrap the whole layout area and show the focused agent's state, and the
//! sidebar highlight, which is the focused agent's own row and no other.

use gpui_kit::Modifiers;
use gpui_kit::TestAppContext;
use knot_core::LayoutMode;

use super::layout_select::focused_agent;
use super::layout_test_support::Fixture;
use super::layout_test_support::Spec;
use super::layout_test_support::open;

/// The selector of the git panel reading `folder`.
fn git_panel(folder: &str) -> &'static str {
    Box::leak(format!("git-panel-folder:{folder}").into_boxed_str())
}

fn folder_of(f: &mut Fixture, name: &str) -> String {
    let id = f.id(name);
    f.view
     .read_with(&f.window, |view, _| view.agent_folder(id))
     .expect("the agent's folder")
}

fn open_git_panel(f: &mut Fixture, name: &str) {
    let id = f.id(name);
    let folder = folder_of(f, name);
    f.view.update(&mut f.window, |view, cx| {
              view.toggle_git_panel(id, &folder, cx);
          });
    f.window.run_until_parked();
}

#[gpui_kit::test]
fn the_git_panel_follows_the_focused_pane(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    open_git_panel(&mut f, "A");
    open_git_panel(&mut f, "B");
    let (a, b) = (folder_of(&mut f, "A"), folder_of(&mut f, "B"));
    assert_ne!(a, b);
    assert!(f.window.debug_bounds(git_panel(&a)).is_some(),
            "A is focused, so its folder's panel shows");
    assert!(f.window.debug_bounds(git_panel(&b)).is_none());

    let inside = f.window.debug_bounds("layout-pane-1").unwrap().center();
    f.window.simulate_click(inside, Modifiers::none());
    f.window.run_until_parked();

    assert!(f.window.debug_bounds(git_panel(&b)).is_some(),
            "focusing pane 1 shows B's folder");
    assert!(f.window.debug_bounds(git_panel(&a)).is_none());
}

#[gpui_kit::test]
fn the_side_panels_sit_beside_the_whole_layout_area(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    let wide = f.window.debug_bounds("layout-area").unwrap();
    open_git_panel(&mut f, "A");
    let folder = folder_of(&mut f, "A");
    let panel = f.window
                 .debug_bounds(git_panel(&folder))
                 .expect("the panel is open");
    let narrow = f.window.debug_bounds("layout-area").unwrap();

    assert!(narrow.size.width < wide.size.width,
            "the panel takes width from the area");
    assert!(narrow.right() <= panel.left() + gpui_kit::px(1.),
            "both panes are left of the panel, not the panel inside one pane");
    let second = f.window.debug_bounds("layout-pane-1").unwrap();
    assert!(second.right() <= panel.left() + gpui_kit::px(1.));
}

#[gpui_kit::test]
fn a_focused_companion_highlights_its_own_row(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[("A", "S")],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "S"],
                             focus:      1, },
                     cx);
    assert!(f.row_highlighted("S"), "the companion's pane is focused");
    assert!(!f.row_highlighted("A"), "not its owner's row");
    assert!(!f.row_highlighted("B"));

    let inside = f.window.debug_bounds("layout-pane-0").unwrap().center();
    f.window.simulate_click(inside, Modifiers::none());
    f.window.run_until_parked();

    assert!(f.row_highlighted("A"));
    assert!(!f.row_highlighted("S"));
}

/// Another thread - an agent over MCP - can move the focus between the
/// frame's selection sync and the panes being drawn. The panes draw from the
/// layout the selection was synced to, so the two still agree, and the
/// change shows on the next frame.
#[gpui_kit::test]
fn a_focus_change_mid_frame_waits_for_the_next_frame(cx: &mut TestAppContext) {
    let mut f = open(&Spec { agents:     &["A", "B"],
                             companions: &[],
                             mode:       LayoutMode::SplitVertical,
                             panes:      &["A", "B"],
                             focus:      0, },
                     cx);
    let (a, b, workspace) = (f.id("A"), f.id("B"), f.workspace);
    let store = f.store.clone();

    let (selected, drawn) = f.view.update(&mut f.window, |view, _| {
                                      view.sync_selection_with_layout();
                                      store.lock().set_workspace_focused_pane(workspace, 1);
                                      let drawn = view.take_frame_layout();
                                      (view.selected_agent, focused_agent(&drawn))
                                  });
    assert_eq!(selected, Some(a));
    assert_eq!(drawn,
               Some(a),
               "the panes draw the layout the selection came from");

    f.view.update(&mut f.window, |_, cx| cx.notify());
    f.window.run_until_parked();
    assert_eq!(f.selected().as_deref(), Some("B"));
    assert_eq!(f.view.read_with(&f.window, |view, _| view.selected_agent),
               Some(b));
}

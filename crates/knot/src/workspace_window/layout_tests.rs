//! A divider drag in a real window: the ratio follows the pointer live,
//! stops at the bound, and reaches disk once, on release.
//!
//! No panes are drawn by the layout yet (task 4.1), so the window still
//! shows the one selected agent under the divider. What is driven here is
//! the divider and the drag state, which do not depend on the panes.
//!
//! Each `Settings` is rooted at a temporary directory, as
//! `tests/workspace_window_open.rs` requires: `persist_workspace_ui` writes
//! the document, and a default `Settings` would write the developer's own.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::Modifiers;
use gpui_kit::MouseButton;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;
use knot_core::LayoutMode;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::consts;
use crate::tests::workspace;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

const WINDOW: (f32, f32) = (1000., 700.);

struct Fixture {
    store:        Arc<Mutex<knot_agents::AgentStore>>,
    workspace_id: Uuid,
    document:     PathBuf,
    cx:           VisualTestContext,
}

/// A window over a workspace of two agents, laid out as `mode` at the
/// centre ratios.
fn open_layout(mode: LayoutMode, cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = workspace("Layout");
    let workspace_id = space.id;
    store.add_workspace(space);
    let agents: Vec<Uuid> = ["A", "B", "C"].iter()
                                           .map(|name| {
                                               store.create(format!("/tmp/{name}"),
                                        knot_agents::CreateOptions { name: Some((*name).into()),
                                                                     workspace_id:
                                                                         Some(workspace_id),
                                                                     ..Default::default() })
                                           })
                                           .collect();
    store.set_workspace_layout(workspace_id, mode, agents);
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));

    let dir = tempfile::tempdir().expect("a temporary settings root");
    let settings = knot_core::Settings::with_store_root(dir.path());
    let document = dir.path().join(knot_core::consts::WORKSPACE_UI_STATE_FILE);
    // The directory has to outlive the test; the OS reaps it.
    std::mem::forget(dir);

    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       WindowRegistry::install(cx);
                       crate::settings_global::install(settings, cx);
                       WorkspaceWindow::open(Arc::clone(&store),
                                             messages,
                                             workspace_id,
                                             crate::workspace_window::OpenRoute::Manager,
                                             cx);
                       cx.windows()[0]
                   });
    let cx = VisualTestContext::from_window(window, cx);
    cx.simulate_resize(size(px(WINDOW.0), px(WINDOW.1)));
    cx.run_until_parked();
    Fixture { store,
              workspace_id,
              document,
              cx }
}

impl Fixture {
    fn ratios(&self) -> (f64, Option<f64>) {
        let ui = self.store.lock().workspace_ui(self.workspace_id);
        (ui.split_ratio, ui.split_ratio_secondary)
    }

    fn divider_centre(&mut self, selector: &'static str) -> gpui_kit::Point<gpui_kit::Pixels> {
        self.cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} was never painted"))
            .center()
    }

    /// What the UI-state document holds on disk, or nothing if it has not
    /// been written.
    fn saved(&self) -> Option<String> {
        std::fs::read_to_string(&self.document).ok()
    }
}

/// The ratio the UI-state document on disk holds for `workspace_id`.
fn saved_split_ratio(document: &str, workspace_id: Uuid) -> f64 {
    let parsed: serde_json::Value = serde_json::from_str(document).expect("valid JSON");
    parsed[workspace_id.to_string()]["splitRatio"].as_f64()
                                                  .expect("a saved split ratio")
}

#[gpui_kit::test]
fn single_mode_draws_no_divider(cx: &mut TestAppContext) {
    let mut fixture = open_layout(LayoutMode::Single, cx);
    assert!(fixture.cx.debug_bounds("layout-divider-primary").is_none());
}

#[gpui_kit::test]
fn dragging_a_split_divider_past_the_bound_clamps_and_persists_once(cx: &mut TestAppContext) {
    let mut fixture = open_layout(LayoutMode::SplitVertical, cx);
    let hit = fixture.cx
                     .debug_bounds("layout-divider-primary")
                     .expect("a split draws its divider");
    assert_eq!(f32::from(hit.size.width),
               consts::DIVIDER_HIT_WIDTH,
               "the hit area is 12px wide");
    let before = fixture.saved();

    let grab = hit.center();
    fixture.cx
           .simulate_mouse_down(grab, MouseButton::Left, Modifiers::none());
    // Well past the left quarter of the area, in two moves: the first lands
    // inside the bounds, the second beyond them.
    fixture.cx
           .simulate_mouse_move(point(grab.x - px(100.), grab.y),
                                MouseButton::Left,
                                Modifiers::none());
    let (live, _) = fixture.ratios();
    assert!(live < 0.5 && live > knot_core::consts::WORKSPACE_SPLIT_RATIO_MIN,
            "the ratio follows the pointer live, got {live}");
    fixture.cx
           .simulate_mouse_move(point(px(5.), grab.y), MouseButton::Left, Modifiers::none());
    assert_eq!(fixture.ratios().0,
               knot_core::consts::WORKSPACE_SPLIT_RATIO_MIN);
    assert_eq!(fixture.saved(),
               before,
               "nothing is written while the drag is running");

    fixture.cx
           .simulate_mouse_up(point(px(5.), grab.y), MouseButton::Left, Modifiers::none());
    fixture.cx.run_until_parked();

    assert_eq!(fixture.ratios().0, 0.25);
    let saved = fixture.saved().expect("the release wrote the document");
    assert_eq!(saved_split_ratio(&saved, fixture.workspace_id), 0.25);

    // The drag is over: neither a further move nor a second release writes.
    std::fs::remove_file(&fixture.document).expect("the document exists");
    fixture.cx.simulate_mouse_move(point(px(500.), grab.y),
                                   MouseButton::Left,
                                   Modifiers::none());
    fixture.cx.simulate_mouse_up(point(px(500.), grab.y),
                                 MouseButton::Left,
                                 Modifiers::none());
    fixture.cx.run_until_parked();
    assert_eq!(fixture.ratios().0, 0.25, "a finished drag does not move it");
    assert!(fixture.saved().is_none(), "a finished drag does not write");
}

#[gpui_kit::test]
fn the_secondary_divider_moves_only_the_secondary_ratio(cx: &mut TestAppContext) {
    let mut fixture = open_layout(LayoutMode::ThreePane, cx);
    let grab = fixture.divider_centre("layout-divider-secondary");
    fixture.cx
           .simulate_mouse_down(grab, MouseButton::Left, Modifiers::none());
    fixture.cx.simulate_mouse_move(point(grab.x, px(WINDOW.1)),
                                   MouseButton::Left,
                                   Modifiers::none());
    fixture.cx.simulate_mouse_up(point(grab.x, px(WINDOW.1)),
                                 MouseButton::Left,
                                 Modifiers::none());
    fixture.cx.run_until_parked();

    let (primary, secondary) = fixture.ratios();
    assert_eq!(primary, 0.5, "the primary ratio is untouched");
    assert_eq!(secondary,
               Some(knot_core::consts::WORKSPACE_SPLIT_RATIO_MAX));
}

/// A release the window never heard leaves the button up on the next move,
/// which ends the drag the same way a release does.
#[gpui_kit::test]
fn a_move_with_the_button_up_ends_the_drag(cx: &mut TestAppContext) {
    let mut fixture = open_layout(LayoutMode::SplitVertical, cx);
    let grab = fixture.divider_centre("layout-divider-primary");
    fixture.cx
           .simulate_mouse_down(grab, MouseButton::Left, Modifiers::none());
    fixture.cx
           .simulate_mouse_move(point(grab.x + px(50.), grab.y),
                                MouseButton::Left,
                                Modifiers::none());
    let dragged = fixture.ratios().0;
    assert!(dragged > 0.5, "the drag moved the ratio, got {dragged}");
    fixture.cx
           .simulate_mouse_move(point(grab.x + px(200.), grab.y), None, Modifiers::none());
    fixture.cx.run_until_parked();

    assert_eq!(fixture.ratios().0, dragged, "the ratio stops where it was");
    let saved = fixture.saved()
                       .expect("the drag was persisted when it ended");
    assert_eq!(saved_split_ratio(&saved, fixture.workspace_id), dragged);
    // And it stays ended: the pointer is down again somewhere else.
    fixture.cx
           .simulate_mouse_move(point(grab.x + px(300.), grab.y),
                                MouseButton::Left,
                                Modifiers::none());
    assert_eq!(fixture.ratios().0, dragged);
}

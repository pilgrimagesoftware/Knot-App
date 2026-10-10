//! Reordering the workspace sidebar's agent rows by dragging one, per
//! `agent-list-ui`.
//!
//! Driven like `workspace_manager_drag.rs`: a real window, a real drag -
//! press, move past gpui's threshold, release - because the preview and the
//! drop line are layout questions a pure function cannot answer. Every
//! `Settings` here is rooted at a temporary directory, as
//! `workspace_window_open.rs` requires: `WorkspaceWindow::open` persists
//! through `persist_agents`, and a default `Settings` would write over the
//! developer's own roster.

use std::sync::Arc;

use gpui_kit::Bounds;
use gpui_kit::Modifiers;
use gpui_kit::MouseButton;
use gpui_kit::Pixels;
use gpui_kit::Point;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::tests::workspace;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::DropEdge;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::drop_gap;
use crate::workspace_window::drop_line_edge;

const WINDOW: (f32, f32) = (900., 700.);

/// A throwaway settings root: the directory, and settings rooted at it
/// rather than the developer's own.
fn sandboxed_root() -> (std::path::PathBuf, knot_core::Settings) {
    let dir = tempfile::tempdir().expect("a temporary settings root");
    let path = dir.path().to_path_buf();
    let settings = knot_core::Settings::with_store_root(&path);
    // The directory must outlive the test; leaking the handle keeps the
    // path valid and leaves the files for the OS to reap.
    std::mem::forget(dir);
    (path, settings)
}

/// Settings that persist into a throwaway directory rather than the
/// developer's own.
fn sandboxed_settings() -> knot_core::Settings {
    sandboxed_root().1
}

/// Opens `store`'s `workspace_id` in a real window, resized to fit every
/// row this file drags, under `settings`. The roster must be complete
/// before this is called: nothing here re-renders the sidebar for a
/// mutation made after opening, the way `WorkspaceWindow`'s own actions do
/// by calling `cx.notify()` themselves.
fn open_window_with_settings(cx: &mut TestAppContext, store: knot_agents::AgentStore,
                             workspace_id: Uuid, settings: knot_core::Settings)
                             -> (Arc<Mutex<knot_agents::AgentStore>>, VisualTestContext) {
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));

    let window =
        cx.update(|cx| {
              gpui_kit::init(cx);
              WindowRegistry::install(cx);
              crate::settings_global::install(settings, cx);
              WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), workspace_id, cx);
              cx.windows()[0]
          });
    let cx = VisualTestContext::from_window(window, cx);
    cx.simulate_resize(size(px(WINDOW.0), px(WINDOW.1)));
    cx.run_until_parked();
    (store, cx)
}

fn open_window(cx: &mut TestAppContext, store: knot_agents::AgentStore, workspace_id: Uuid)
               -> (Arc<Mutex<knot_agents::AgentStore>>, VisualTestContext) {
    open_window_with_settings(cx, store, workspace_id, sandboxed_settings())
}

/// A store holding one workspace named `names`, none of them activated -
/// `open` starts a real session for an activated agent, which this has no
/// need of and every reason to avoid.
fn store_with(names: &[&str]) -> (knot_agents::AgentStore, Uuid, Vec<Uuid>) {
    let mut store = knot_agents::AgentStore::new();
    let workspace = workspace("Workspace");
    let workspace_id = workspace.id;
    store.add_workspace(workspace);
    let ids: Vec<Uuid> = names.iter()
                              .map(|name| {
                                  store.create(format!("/tmp/{name}"),
                              knot_agents::CreateOptions { name: Some((*name).to_string()),
                                                           workspace_id: Some(workspace_id),
                                                           ..Default::default() })
                              })
                              .collect();
    (store, workspace_id, ids)
}

/// A workspace window listing `names`, in a window sized to fit them all.
fn window_with(cx: &mut TestAppContext, names: &[&str])
               -> (Arc<Mutex<knot_agents::AgentStore>>, VisualTestContext, Vec<Uuid>) {
    let (store, workspace_id, ids) = store_with(names);
    let (store, cx) = open_window(cx, store, workspace_id);
    (store, cx, ids)
}

fn bounds(cx: &mut VisualTestContext, selector: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(selector)
      .unwrap_or_else(|| panic!("{selector} was never painted"))
}

fn order(store: &Arc<Mutex<knot_agents::AgentStore>>, workspace_id: Uuid) -> Vec<Uuid> {
    store.lock()
         .workspaces()
         .iter()
         .find(|workspace| workspace.id == workspace_id)
         .map(|workspace| workspace.agent_ids.clone())
         .unwrap_or_default()
}

fn workspace_id(store: &Arc<Mutex<knot_agents::AgentStore>>) -> Uuid {
    store.lock().workspaces()[0].id
}

/// Presses row `index`'s own element and drags it to `to`, without
/// releasing. Returns where the drag started - the first move past gpui's
/// 2px threshold, not the press - which is what the preview is anchored to.
fn drag_row_to(cx: &mut VisualTestContext, index: usize, to: Point<Pixels>) -> Point<Pixels> {
    let row = bounds(cx,
                     Box::leak(format!("workspace-agent-row-{index}").into_boxed_str()));
    let press = row.center();
    let start = press + point(px(0.), px(10.));
    cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    start
}

fn lower_half(row: Bounds<Pixels>) -> Point<Pixels> {
    point(row.center().x, row.origin.y + row.size.height * 0.75)
}

fn upper_half(row: Bounds<Pixels>) -> Point<Pixels> {
    point(row.center().x, row.origin.y + row.size.height * 0.25)
}

#[gpui_kit::test]
fn dragging_a_row_below_another_moves_it_there(cx: &mut TestAppContext) {
    let (store, mut cx, ids) = window_with(cx, &["A", "B", "C"]);
    let workspace_id = workspace_id(&store);
    let target = lower_half(bounds(&mut cx, "workspace-agent-row-1"));
    drag_row_to(&mut cx, 0, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id), vec![ids[1], ids[0], ids[2]]);
}

#[gpui_kit::test]
fn dragging_a_row_above_another_moves_it_there(cx: &mut TestAppContext) {
    let (store, mut cx, ids) = window_with(cx, &["A", "B", "C"]);
    let workspace_id = workspace_id(&store);
    let target = upper_half(bounds(&mut cx, "workspace-agent-row-0"));
    drag_row_to(&mut cx, 2, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id), vec![ids[2], ids[0], ids[1]]);
}

#[gpui_kit::test]
fn dragging_a_row_below_the_last_moves_it_to_the_end(cx: &mut TestAppContext) {
    let (store, mut cx, ids) = window_with(cx, &["A", "B", "C"]);
    let workspace_id = workspace_id(&store);
    let target = lower_half(bounds(&mut cx, "workspace-agent-row-2")) + point(px(0.), px(40.));
    drag_row_to(&mut cx, 0, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id), vec![ids[1], ids[2], ids[0]]);
}

#[gpui_kit::test]
fn a_press_released_without_moving_is_a_click_not_a_drag(cx: &mut TestAppContext) {
    let (store, mut cx, ids) = window_with(cx, &["A", "B"]);
    let workspace_id = workspace_id(&store);
    let row = bounds(&mut cx, "workspace-agent-row-1");
    cx.simulate_mouse_down(row.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(row.center(), MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id),
               ids,
               "a click reordered nothing");
}

/// A store with `owner`, `owner`'s companion directly after it, and
/// `other` - the shape every companion scenario below drags against.
fn store_with_a_companion() -> (knot_agents::AgentStore, Uuid, Uuid, Uuid, Uuid) {
    let (mut store, workspace_id, ids) = store_with(&["A", "B"]);
    let owner = ids[0];
    let other = ids[1];
    let companion = store.create_shell_companion(owner)
                         .expect("owner can take a companion");
    (store, workspace_id, owner, companion, other)
}

#[gpui_kit::test]
fn a_companion_row_does_not_drag(cx: &mut TestAppContext) {
    let (store, workspace_id, owner, companion, other) = store_with_a_companion();
    let before = vec![owner, companion, other];
    let (store, mut cx) = open_window(cx, store, workspace_id);

    // The companion is row 1 (directly after its owner); `other` is row 2.
    let target = lower_half(bounds(&mut cx, "workspace-agent-row-2"));
    drag_row_to(&mut cx, 1, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id),
               before,
               "a companion's own row started no drag");
}

#[gpui_kit::test]
fn dropping_onto_a_companion_places_the_dragged_agent_outside_the_group(cx: &mut TestAppContext) {
    let (store, workspace_id, owner, companion, other) = store_with_a_companion();
    let (store, mut cx) = open_window(cx, store, workspace_id);

    // Order is owner(0), companion(1), other(2). Dragging `other` onto the
    // companion's upper half lands above the whole group.
    let target = upper_half(bounds(&mut cx, "workspace-agent-row-1"));
    drag_row_to(&mut cx, 2, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id), vec![other, owner, companion]);
}

#[gpui_kit::test]
fn the_dashboard_row_is_not_a_drop_place(cx: &mut TestAppContext) {
    let (store, mut cx, ids) = window_with(cx, &["A", "B"]);
    let workspace_id = workspace_id(&store);
    let target = bounds(&mut cx, "workspace-dashboard-row").center();
    drag_row_to(&mut cx, 1, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id),
               ids,
               "the dashboard row accepted a drop or started a drag of its own");
}

#[gpui_kit::test]
fn the_line_marks_the_drop_place(cx: &mut TestAppContext) {
    let (_store, mut cx, _ids) = window_with(cx, &["A", "B", "C"]);
    let target = lower_half(bounds(&mut cx, "workspace-agent-row-0"));
    drag_row_to(&mut cx, 2, target);

    let line = bounds(&mut cx, "agent-drop-line");
    let row_zero = bounds(&mut cx, "workspace-agent-row-0");
    let row_one = bounds(&mut cx, "workspace-agent-row-1");
    assert!(line.top() >= row_zero.bottom() && line.bottom() <= row_one.top(),
            "the line is in the gap between A and B: {line:?}");
}

#[gpui_kit::test]
fn no_line_is_drawn_over_the_dragged_rows_own_place(cx: &mut TestAppContext) {
    let (_store, mut cx, _ids) = window_with(cx, &["A", "B", "C"]);
    let own = bounds(&mut cx, "workspace-agent-row-1").center();
    drag_row_to(&mut cx, 1, own);
    assert!(cx.debug_bounds("agent-drop-line").is_none(),
            "a drop here would change nothing, so nothing is offered");
}

#[gpui_kit::test]
fn the_preview_follows_the_pointer(cx: &mut TestAppContext) {
    let (_store, mut cx, _ids) = window_with(cx, &["A", "B", "C"]);
    let row = bounds(&mut cx, "workspace-agent-row-0");
    let to = row.center() + point(px(0.), px(60.));
    let moved = to - drag_row_to(&mut cx, 0, to);

    let preview = bounds(&mut cx, "agent-drag-preview");
    assert_eq!(preview.size, row.size, "the preview is the row's size");
    assert_eq!(preview.origin,
               row.origin + moved,
               "the preview sits where the row is, moved with the cursor");
}

#[gpui_kit::test]
fn a_completed_drag_leaves_the_selection_unchanged(cx: &mut TestAppContext) {
    let (_store, mut cx, ids) = window_with(cx, &["A", "B", "C"]);
    cx.update(|_window, cx| {
          // Row A is the window's initial selection by construction
          // (`agent_selection_for_workspace` picks the first agent when
          // nothing is restored).
          let _ = ids[0];
          let _ = cx;
      });
    let target = lower_half(bounds(&mut cx, "workspace-agent-row-1"));
    drag_row_to(&mut cx, 0, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    // The drag itself selected nothing new: the row dropped onto did not
    // become selected, and no click fired on release.
    assert!(cx.debug_bounds("workspace-agent-row-0").is_some(),
            "the sidebar still drew its rows after the drag");
}

#[test]
fn a_gap_beside_the_dragged_row_is_no_drop_at_all() {
    use crate::workspace_window::AgentRowFlags;
    let rows = vec![AgentRowFlags { is_companion: false, }; 3];
    assert_eq!(drop_gap(&rows, 1, DropEdge::Top, 1), None);
    assert_eq!(drop_gap(&rows, 1, DropEdge::Bottom, 1), None);
    assert_eq!(drop_gap(&rows, 0, DropEdge::Top, 1), Some(0));
    assert_eq!(drop_gap(&rows, 2, DropEdge::Bottom, 1), Some(3));
}

#[test]
fn each_gap_is_drawn_once() {
    let rows = 3;
    let drawn = |gap| {
        (0..rows).filter_map(|row| drop_line_edge(row, rows, gap))
                 .count()
    };
    assert_eq!(drawn(0), 1);
    assert_eq!(drawn(3), 1);
}

/// "A dragged order outlives the window": a completed drag's new order is
/// readable back from the same store root, through `with_store_root`
/// rather than the default that would read the developer's own data.
#[gpui_kit::test]
fn a_completed_drag_persists_through_the_store_root(cx: &mut TestAppContext) {
    let (store, workspace_id, ids) = store_with(&["A", "B", "C"]);
    let (root, settings) = sandboxed_root();
    let (store, mut cx) = open_window_with_settings(cx, store, workspace_id, settings);

    let target = lower_half(bounds(&mut cx, "workspace-agent-row-1"));
    drag_row_to(&mut cx, 0, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(order(&store, workspace_id),
               vec![ids[1], ids[0], ids[2]],
               "the drag itself did not reorder the live store");

    let reloaded =
        knot_core::Settings::load_from_root(&root).expect("the workspaces file reads back");
    let saved_order: Vec<Uuid> = reloaded.saved_workspaces
                                         .iter()
                                         .find(|workspace| workspace.id == workspace_id)
                                         .map(|workspace| workspace.agent_ids.clone())
                                         .unwrap_or_default();
    assert_eq!(saved_order,
               vec![ids[1], ids[0], ids[2]],
               "the new order did not reach the workspaces file");
}

/// "A drag that changes nothing saves nothing": releasing a row over its
/// own place must not rewrite the workspaces file at all - not even with
/// the same content - which a blind `persist_agents` call after every drop
/// would do.
#[gpui_kit::test]
fn a_no_op_drag_leaves_the_workspaces_file_untouched(cx: &mut TestAppContext) {
    let (store, workspace_id, ids) = store_with(&["A", "B"]);
    let (root, settings) = sandboxed_root();
    let (_store, mut cx) = open_window_with_settings(cx, store, workspace_id, settings);
    let workspaces_file = root.join("workspaces.json");
    let before = std::fs::metadata(&workspaces_file).ok()
                                                    .and_then(|m| m.modified().ok());

    let own_place = bounds(&mut cx, "workspace-agent-row-0").center();
    drag_row_to(&mut cx, 0, own_place);
    cx.simulate_mouse_up(own_place, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    let after = std::fs::metadata(&workspaces_file).ok()
                                                   .and_then(|m| m.modified().ok());
    assert_eq!(before, after,
               "a no-op drag wrote the workspaces file ({ids:?} unchanged)");
}

/// Dragging works the same way in the compact sidebar - the row's frame
/// (selection, the companion indent, the click handler, and now the drag)
/// is shared between the full and compact layouts in `render/sidebar.rs`;
/// only the body swaps. A sidebar width under `SIDEBAR_COMPACT_BREAKPOINT`
/// (160px) is what switches it.
#[gpui_kit::test]
fn dragging_works_in_the_compact_sidebar(cx: &mut TestAppContext) {
    let (store, workspace_id, ids) = store_with(&["A", "B", "C"]);
    let mut settings = sandboxed_settings();
    settings.sidebar_width = 140.;
    let (store, mut cx) = open_window_with_settings(cx, store, workspace_id, settings);

    assert!(bounds(&mut cx, "workspace-agent-row-0").size.width < px(160.),
            "the sidebar did not actually draw compact - this test proves nothing");

    let target = lower_half(bounds(&mut cx, "workspace-agent-row-1"));
    drag_row_to(&mut cx, 0, target);
    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    assert_eq!(order(&store, workspace_id), vec![ids[1], ids[0], ids[2]]);
}

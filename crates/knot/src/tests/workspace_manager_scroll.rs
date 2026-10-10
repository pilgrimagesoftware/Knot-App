//! The Workspace Manager's roster scrolls once it outgrows the window.
//!
//! The rows were a plain column under a `flex_1` parent with no `min_h_0`,
//! so the column took its content's height, ran past the bottom of the
//! window and was clipped there - the rows below the fold could not be
//! reached. The question is about layout, so it is asked of a drawn window:
//! the list's painted bounds must end inside it.
//!
//! Contract: `openspec/specs/workspace-manager-ui/spec.md`, "The workspace
//! list scrolls". A row is visible when its painted bounds lie inside the
//! list's viewport - the `workspace-manager-list` selector, which sits on the
//! viewport rather than on the scrolled column of rows.

use gpui_kit::Bounds;
use gpui_kit::Pixels;
use gpui_kit::ScrollDelta;
use gpui_kit::ScrollWheelEvent;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;

use crate::tests::workspace;
use crate::tests::workspace_dialog::manager;

/// Short enough that `WORKSPACE_COUNT` rows cannot fit in it.
const WINDOW_HEIGHT: f32 = 400.;

const WORKSPACE_COUNT: usize = 40;

#[gpui_kit::test]
fn a_roster_taller_than_the_window_stays_inside_it(cx: &mut TestAppContext) {
    let (store, mut cx, _manager) = manager(cx);
    {
        let mut store = store.lock();
        for index in 0..WORKSPACE_COUNT {
            store.add_workspace(workspace(&format!("Workspace {index}")));
        }
    }
    cx.simulate_resize(size(px(600.), px(WINDOW_HEIGHT)));
    cx.run_until_parked();

    let list = cx.debug_bounds("workspace-manager-list")
                 .expect("the workspace list was never painted");
    assert!(list.bottom() <= px(WINDOW_HEIGHT),
            "the list ends at {:?}, past the window's {WINDOW_HEIGHT}px, so its lower rows are \
             clipped rather than scrolled to",
            list.bottom());
}

/// `count` workspaces in a window `WINDOW_HEIGHT` tall, drawn. Returns the
/// window and how many rows the list holds - the store may already carry a
/// default workspace of its own.
fn manager_with(count: usize, cx: &mut TestAppContext) -> (VisualTestContext, usize) {
    let (store, cx, _manager) = manager(cx);
    let rows = {
        let mut store = store.lock();
        for index in 0..count {
            store.add_workspace(workspace(&format!("Workspace {index}")));
        }
        store.workspaces().len()
    };
    cx.simulate_resize(size(px(600.), px(WINDOW_HEIGHT)));
    cx.run_until_parked();
    (cx, rows)
}

/// The list's viewport: the area rows are visible in.
fn viewport(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.debug_bounds("workspace-manager-list")
      .expect("the workspace list was never painted")
}

/// Row `row`'s painted bounds, or `None` when it was not painted at all.
fn row_bounds(cx: &mut VisualTestContext, row: usize) -> Option<Bounds<Pixels>> {
    // `debug_bounds` takes a `'static` selector; a test leaking a few short
    // strings costs nothing.
    let selector: &'static str = Box::leak(format!("workspace-row-{row}").into_boxed_str());
    cx.debug_bounds(selector)
}

fn is_visible(row: Bounds<Pixels>, viewport: Bounds<Pixels>) -> bool {
    row.top() >= viewport.top() && row.bottom() <= viewport.bottom()
}

/// "The last workspace is reachable": scrolled to its end, a list longer
/// than the window shows its last row.
#[gpui_kit::test]
fn scrolling_to_the_end_shows_the_last_workspace(cx: &mut TestAppContext) {
    let (mut cx, rows) = manager_with(WORKSPACE_COUNT, cx);
    let last = rows - 1;
    let list = viewport(&mut cx);
    assert!(row_bounds(&mut cx, last).is_none_or(|bounds| !is_visible(bounds, list)),
            "the last row is visible before any scrolling, so this proves nothing");

    // Far more than the list's overflow, so it lands at the end however tall
    // a row is.
    let over_the_list = list.center();
    for _ in 0..40 {
        cx.simulate_event(ScrollWheelEvent { position: over_the_list,
                                             delta: ScrollDelta::Pixels(point(px(0.),
                                                                              px(-200.))),
                                             ..Default::default() });
    }
    cx.run_until_parked();

    let list = viewport(&mut cx);
    let last_row = row_bounds(&mut cx, last).expect("the last row was not painted after scrolling");
    assert!(is_visible(last_row, list),
            "scrolled to the end, the last row ({last_row:?}) is still outside the list ({list:?})");
}

/// "Rows that fit need no scroll": with a few workspaces, every row is
/// visible as drawn.
#[gpui_kit::test]
fn a_short_roster_shows_every_workspace_without_scrolling(cx: &mut TestAppContext) {
    let (mut cx, rows) = manager_with(3, cx);
    let list = viewport(&mut cx);
    for row in 0..rows {
        let bounds =
            row_bounds(&mut cx, row).unwrap_or_else(|| panic!("row {row} was not painted"));
        assert!(is_visible(bounds, list),
                "row {row} ({bounds:?}) is outside the list ({list:?}) with nothing scrolled");
    }
}

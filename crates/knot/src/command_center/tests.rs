//! The Command Center's store poll (#17): a card's status text written where
//! no notify reaches still repaints the window, once.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use gpui_kit::AppContext;
use gpui_kit::TestAppContext;
use parking_lot::Mutex;

use super::CommandCenterWindow;
use super::spawn_store_poll;
use crate::consts;
use crate::diff_stats::DiffStatsCache;
use crate::tests::workspace;

#[gpui_kit::test]
fn status_text_written_off_the_ui_thread_repaints_the_cards(cx: &mut TestAppContext) {
    let mut store = knot_agents::AgentStore::new();
    let space = workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let id = store.create("~/agent".to_string(),
                          knot_agents::CreateOptions { workspace_id: Some(workspace_id),
                                                       ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let view = cx.new(|cx| {
                     spawn_store_poll(cx);
                     CommandCenterWindow { store: Arc::clone(&store),
                                           messages,
                                           dashboard_sort: Default::default(),
                                           diff_stats: DiffStatsCache::default(),
                                           drawn_store_revision: 0 }
                 });
    let notified = Arc::new(AtomicUsize::new(0));
    let _observer = cx.update(|cx| {
                          let notified = Arc::clone(&notified);
                          cx.observe(&view, move |_, _| {
                                notified.fetch_add(1, Ordering::SeqCst);
                            })
                      });
    let tick = |cx: &mut TestAppContext| {
        cx.executor().advance_clock(consts::REPAINT_POLL_INTERVAL);
        cx.run_until_parked();
    };
    tick(cx);
    let before = notified.load(Ordering::SeqCst);

    store.lock().set_status_text(id, "reviewing".into());
    tick(cx);
    assert_eq!(notified.load(Ordering::SeqCst),
               before + 1,
               "the new text has to be drawn");

    tick(cx);
    assert_eq!(notified.load(Ordering::SeqCst), before + 1, "once");
}

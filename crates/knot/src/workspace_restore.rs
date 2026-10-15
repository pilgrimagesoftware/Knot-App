//! Reopening the workspaces whose windows were open when the app last quit
//! (issue #245), behind the "Re-open workspaces on launch" setting.
//!
//! The open set is recorded once, at quit, rather than kept current as
//! windows open and close. A window's close cannot tell the user closing it
//! from the app tearing it down, and gpui drops every window during
//! shutdown - so a per-close record would read "nothing open" after every
//! quit. The quit observer runs before that teardown
//! (`App::shutdown` calls it ahead of clearing the window list), so the
//! registry still lists exactly what the user left open.
//!
//! The Ctrl-C path in `quit_on_terminal_signals` exits without running quit
//! observers, so a terminal kill keeps the set from the previous quit. That
//! path is a kill, not a quit request, and is abrupt by design.

use std::sync::Arc;

use gpui_kit::App;
use parking_lot::Mutex;

use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Records the open workspaces when the app quits, whatever the setting:
/// turning it on later then reopens what was open at the last quit rather
/// than whatever was recorded before it was turned off.
pub(crate) fn record_open_workspaces_at_quit(store: Arc<Mutex<knot_agents::AgentStore>>,
                                             cx: &mut App) {
    cx.on_app_quit(move |cx| {
          record_open_workspaces(&store, cx);
          async {}
      })
      .detach();
}

/// Writes which workspaces have a window open into the UI-state document.
fn record_open_workspaces(store: &Mutex<knot_agents::AgentStore>, cx: &App) {
    let open = WindowRegistry::open_workspaces(cx);
    // Scoped so the store guard is released before the write, the same rule
    // `WorkspaceWindow::persist_workspace_ui` follows.
    let ui = {
        let mut store = store.lock();
        if !store.record_open_workspaces(&open) {
            return;
        }
        store.saved_workspace_ui()
    };
    let installed = crate::settings_global::write(cx, |settings| {
        settings.workspace_ui = ui.clone();
    });
    if let Err(error) = installed.persist_workspace_ui() {
        // Nowhere to surface it: the app is exiting. What is lost is which
        // windows the next launch opens.
        eprintln!("failed to record open workspaces: {error}");
    }
}

/// Opens a window for every workspace that had one at the last quit, when
/// the setting is on.
///
/// The store only holds workspaces when `restore_layout_on_launch` restored
/// them, so with that off there is nothing here to reopen.
pub(crate) fn reopen_workspaces(store: &Arc<Mutex<knot_agents::AgentStore>>,
                                messages: &Arc<Mutex<knot_messaging::MessageStore>>,
                                cx: &mut App) {
    if !crate::settings_global::read(cx).reopen_workspaces_on_launch {
        return;
    }
    let reopen = store.lock().workspaces_open_at_quit();
    for workspace_id in reopen {
        WorkspaceWindow::open(Arc::clone(store), Arc::clone(messages), workspace_id, cx);
    }
}

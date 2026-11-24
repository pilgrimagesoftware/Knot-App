//! Closing a workspace window: shutting down every terminal and panel
//! session it still holds.

use super::window::WorkspaceWindow;

impl Drop for WorkspaceWindow {
    /// Tears down both launch paths, not just the terminal one.
    ///
    /// `sessions` holds the PTY-backed shell agents; `panel_sessions` holds
    /// the ACP connections, each owning an adapter subprocess. Only the
    /// former was shut down here, so closing a workspace window left one
    /// orphaned adapter per panel agent - the same gap `remove_session`
    /// documents for a single agent, applied to the whole window.
    fn drop(&mut self) {
        eprintln!("window released: workspace {} window {}",
                  self.workspace_id,
                  self.window_handle.window_id().as_u64());
        for (id, session) in &self.sessions {
            {
                let mut session = session.lock();
                // As in `remove_session`: the drop below is the teardown,
                // this is only the polite half of it.
                if let Err(error) = session.shutdown() {
                    eprintln!("failed to shut down agent {id}'s terminal: {error}");
                }
            }
        }
        // Dropping the slot is what guarantees the teardown: it releases
        // the last `AcpClient`, which releases the transport, whose
        // `kill_on_drop` child then dies. The spawned `stop()` is the
        // polite `session/close` on top of that, and best-effort only -
        // this runtime is itself dropped moments later, so a task that has
        // not started may never run. Nothing depends on it having.
        // Retired, not dropped: a slot still connecting would otherwise
        // finish after the window closed, and its adapter outlive it (#382).
        for slot in std::mem::take(&mut self.panel_sessions).into_values() {
            let handle = slot.lock().retire();
            if let Some(handle) = handle {
                let _runtime_guard = self.runtime.enter();
                self.runtime.spawn(async move { handle.stop().await });
            }
        }
    }
}

//! Carrying a confirmation request to the UI and its answer back.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::oneshot;

use super::ConfirmationRequest;

/// A request waiting for the user, and where its answer goes.
pub struct PendingConfirmation {
    pub request: ConfirmationRequest,
    answer:      oneshot::Sender<bool>,
}

impl PendingConfirmation {
    /// A request and the receiver its answer arrives on - what
    /// [`await_confirmation`] queues, and what a UI test plants.
    pub fn new(request: ConfirmationRequest) -> (Self, oneshot::Receiver<bool>) {
        let (answer, answered) = oneshot::channel();
        (Self { request, answer }, answered)
    }

    /// Answers the request. Dropping it unanswered counts as a denial.
    pub fn answer(self, confirmed: bool) {
        // The tool call may have timed out and gone; then nobody is waiting.
        let _ = self.answer.send(confirmed);
    }
}

/// Requests for the UI to drain and show, one dialog each. Pushed from the
/// MCP server's thread; drained by whichever window repaints first, as the
/// activation queue is.
pub type ConfirmationQueue = Arc<Mutex<Vec<PendingConfirmation>>>;

/// Whether the user confirms `request`: queued on `queue`, then awaited up
/// to `timeout`.
///
/// `false` when there is no queue - no UI to ask, so nothing that needs
/// asking may happen - when the user denies, when the dialog is dropped
/// unanswered, and when `timeout` passes first. Holds no lock while it
/// waits.
pub async fn await_confirmation(queue: Option<&ConfirmationQueue>, request: ConfirmationRequest,
                                timeout: Duration)
                                -> bool {
    let Some(queue) = queue
    else {
        return false;
    };
    let (pending, answered) = PendingConfirmation::new(request);
    queue.lock().push(pending);
    matches!(tokio::time::timeout(timeout, answered).await, Ok(Ok(true)))
}

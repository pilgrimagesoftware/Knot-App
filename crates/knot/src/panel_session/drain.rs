//! The task that folds a session's event stream into its panel state, and
//! saves the session's conversation times as its turns end (#220).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use knot_acp::{SessionEvent, SessionUpdate};
use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;

use super::TimesSaver;
use crate::panel_state::PanelState;
use crate::subagent_feed::SubagentSink;

/// Spawns, on the current tokio runtime, the task draining `events` into
/// `state` until the session ends, setting `dirty` after each event.
pub(super) fn spawn(mut events: UnboundedReceiver<SessionEvent>, state: Arc<Mutex<PanelState>>,
                    dirty: Arc<AtomicBool>, subagents: Option<SubagentSink>,
                    saver: Option<Arc<TimesSaver>>) {
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            let ended = matches!(event, SessionEvent::Ended(_));
            // Before the fold, and by reference, so the event still moves
            // into `apply` intact. The sink writes to the shared subagent
            // registry, which carries its own changed flag - it does not
            // ride this session's dirty bit, because a subagent recorded
            // for an agent whose pane is not shown still has to be there
            // when that pane is next drawn.
            if let Some(sink) = &subagents {
                sink.observe(&event);
            }
            let turn_over =
                ended || matches!(event, SessionEvent::Update(SessionUpdate::TurnEnd { .. }));
            let snapshot = {
                let mut state = state.lock();
                state.apply(event);
                turn_over.then(|| state.times.snapshot())
            };
            // Saved once per turn: a turn's entries are all logged by the
            // time it ends, and a write per streamed chunk would be thousands
            // of them. The snapshot, not the live log - a restored session
            // still replaying has logged nothing live, and saving that would
            // wipe its restored times.
            if let (Some(snapshot), Some(saver)) = (snapshot, &saver) {
                saver.save_in_background(snapshot);
            }
            dirty.store(true, Ordering::SeqCst);
            if ended {
                break;
            }
        }
    });
}

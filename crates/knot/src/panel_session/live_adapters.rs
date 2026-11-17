//! One live adapter per agent, across every window in the app (#456).
//!
//! Each workspace window keeps its own `panel_sessions`, so nothing in a
//! window can see that another window - or a window's view outliving its
//! window - already runs an agent's adapter. Two adapters for one agent load
//! the same conversation and both act on it: every message is read twice and
//! every task done twice. This is the app-wide check that refuses the
//! second.
//!
//! Held weakly: a slot is live while some window still holds it and it is
//! connecting or connected. A retired or failed slot owns no subprocess, so
//! a restart, which retires the old slot and starts a new one straight
//! away, is not refused.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Weak};

use parking_lot::Mutex;
use uuid::Uuid;

use super::{PanelPhase, PanelSessionSlot};

/// The latest slot started for each agent. Process-wide rather than a GPUI
/// global: the window that starts a session only has `&App`, and an agent's
/// id is unique to it, so there is nothing to keep apart per app.
static LIVE: LazyLock<Mutex<Claims>> = LazyLock::new(|| Mutex::new(Claims::default()));

#[derive(Default)]
struct Claims {
    slots:    HashMap<Uuid, Weak<Mutex<PanelSessionSlot>>>,
    /// Agents whose refusal has been logged. A refused window asks again
    /// on every frame it draws the agent's pane, so it is logged once.
    reported: HashSet<Uuid>,
}

/// Records `slot` as `agent`'s adapter, unless another live slot already is.
/// Reports whether the caller may start it.
pub(crate) fn claim(agent: Uuid, slot: &Arc<Mutex<PanelSessionSlot>>) -> bool {
    let mut claims = LIVE.lock();
    if let Some(existing) = claims.slots.get(&agent).and_then(Weak::upgrade)
       && !Arc::ptr_eq(&existing, slot)
       && is_live(&existing.lock())
    {
        if claims.reported.insert(agent) {
            eprintln!("agent {agent}: an adapter is already running for it in another window; \
                       not starting a second");
        }
        return false;
    }
    claims.reported.remove(&agent);
    claims.slots.insert(agent, Arc::downgrade(slot));
    true
}

/// Whether `slot` owns, or is about to own, an adapter subprocess.
fn is_live(slot: &PanelSessionSlot) -> bool {
    match slot.phase() {
        PanelPhase::Connecting(_) | PanelPhase::Ready => true,
        PanelPhase::Failed | PanelPhase::Retired => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;
    use uuid::Uuid;

    use super::claim;
    use crate::panel_session::PanelSessionSlot;

    fn connecting() -> Arc<Mutex<PanelSessionSlot>> {
        Arc::new(Mutex::new(PanelSessionSlot::connecting().0))
    }

    #[test]
    fn a_second_live_slot_for_an_agent_is_refused() {
        let agent = Uuid::new_v4();
        let first = connecting();
        let second = connecting();

        assert!(claim(agent, &first));
        assert!(!claim(agent, &second), "the first is still connecting");
        assert!(claim(Uuid::new_v4(), &second),
                "another agent is unaffected");
    }

    #[test]
    fn a_retired_or_dropped_slot_does_not_block_the_next() {
        let agent = Uuid::new_v4();
        let first = connecting();
        assert!(claim(agent, &first));

        let _ = first.lock().retire();
        let restarted = connecting();
        assert!(claim(agent, &restarted), "a restart retires, then starts");

        drop(restarted);
        assert!(claim(agent, &connecting()),
                "a window that let go holds nothing");
    }
}

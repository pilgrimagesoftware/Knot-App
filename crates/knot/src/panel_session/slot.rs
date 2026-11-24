//! The lifecycle slot a window holds for a Panel-mode agent: connecting,
//! ready, failed or retired, the `Connecting` state's retirement signal, and
//! the phase summary the UI's repaint poll compares. It owns no connection
//! logic itself; `connect_into` (see [`super::connect`]) publishes into it.

use std::sync::Arc;

use knot_acp::ConfigOption;
use knot_terminal::{ConnectProgress, ConnectStep};
use parking_lot::Mutex;

use super::{AgentIdentity, PanelSessionHandle};

/// A Panel-mode agent's connection lifecycle: connecting, ready to use, or
/// failed to connect (per `acp-client`'s "fails closed to Terminal mode
/// with a visible error" requirement - the caller renders `Failed`'s
/// message rather than hanging on `Connecting` forever).
pub enum PanelSessionSlot {
    /// Connecting, carrying the step the attempt is currently on so the
    /// placeholder can say what it's waiting for, and the signal that
    /// cancels the attempt when the slot is retired.
    Connecting(Connecting),
    /// Boxed so the slot is a pointer wide in every phase: the handle is by
    /// far the largest variant and had reached clippy's
    /// `large_enum_variant` bound exactly.
    Ready(Box<PanelSessionHandle>),
    Failed(String),
    /// Torn down by the window. Terminal: [`super::connect_into`] publishes
    /// nothing into a retired slot, and an adapter it had already started
    /// is stopped rather than left running with no pane (#382).
    Retired,
}

/// A `Connecting` slot's contents.
///
/// Holds both ends of the retirement signal. [`super::connect_into`] takes the
/// receiver when it starts, and the sender stays here, so replacing this
/// variant - with `Retired`, or with the attempt's own outcome - is what
/// fires it. Retiring a slot therefore cancels its connect by construction,
/// with no flag for a teardown path to forget to set.
pub struct Connecting {
    progress: ConnectProgress,
    retired:  Option<tokio::sync::oneshot::Receiver<()>>,
    _retire:  tokio::sync::oneshot::Sender<()>,
}

impl Connecting {
    /// The step the attempt is on, for the placeholder.
    pub fn progress(&self) -> &ConnectProgress {
        &self.progress
    }
}

impl PanelSessionSlot {
    /// A fresh `Connecting` slot and the progress cell the connect task
    /// writes to - the two halves of the same handle, so the caller can't
    /// accidentally hand the task a cell the slot isn't watching.
    pub fn connecting() -> (Self, ConnectProgress) {
        let progress: ConnectProgress =
            Arc::new(Mutex::new(ConnectStep::Starting { program: "the agent", }));
        let (retire, retired) = tokio::sync::oneshot::channel();
        (Self::Connecting(Connecting { progress: Arc::clone(&progress),
                                       retired:  Some(retired),
                                       _retire:  retire, }),
         progress)
    }

    /// Retires the slot, handing back the session it held if it was
    /// `Ready` so the caller can stop its adapter.
    ///
    /// The one way a window lets go of a slot (#382). A `Connecting` slot
    /// needs nothing back: retiring it fires the signal its connect task
    /// waits on, which drops the half-made connection and the adapter with
    /// it, and makes the task publish nothing. Teardown used to drop the
    /// slot, swapping in a fresh `Connecting` placeholder, which left that
    /// task running: it finished connecting, published into a slot no
    /// window held, and ran the registration turn on an adapter nobody
    /// could see, beside the one the next start spawned.
    pub fn retire(&mut self) -> Option<Box<PanelSessionHandle>> {
        match std::mem::replace(self, Self::Retired) {
            Self::Ready(handle) => Some(handle),
            Self::Connecting(_) | Self::Failed(_) | Self::Retired => None,
        }
    }

    /// The retirement signal of a slot still `Connecting`, taken once by
    /// the connect task. `None` when the slot is retired already, or when
    /// another task has claimed it - either way, nothing for this attempt
    /// to do.
    pub(super) fn take_retire_signal(&mut self) -> Option<tokio::sync::oneshot::Receiver<()>> {
        match self {
            Self::Connecting(connecting) => connecting.retired.take(),
            Self::Ready(_) | Self::Failed(_) | Self::Retired => None,
        }
    }

    /// Replaces a `Connecting` slot with the attempt's outcome. Hands
    /// `outcome` back when the slot is no longer `Connecting` - retired
    /// while the attempt ran - so the caller can stop what it started.
    ///
    /// The check and the write are one critical section: a retire on the
    /// UI thread lands either before it, and the outcome comes back, or
    /// after it, and retires the published handle. There is no third order
    /// in which an adapter ends up in a slot no window holds.
    pub(super) fn publish(slot: &Mutex<Self>, outcome: Self) -> std::result::Result<(), Self> {
        let mut slot = slot.lock();
        if matches!(*slot, Self::Connecting(_)) {
            *slot = outcome;
            Ok(())
        }
        else {
            Err(outcome)
        }
    }

    /// Whether the slot is still waiting for this attempt.
    pub(super) fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting(_))
    }

    /// Which lifecycle phase this slot is in, without borrowing the
    /// handle. The UI's repaint poll compares this against the phase it
    /// last drew: the connecting task writes the slot from a background
    /// thread, and neither `Connecting` nor `Failed` carries a dirty flag
    /// of its own, so without this a failed connection would leave the
    /// pane showing "Connecting to agent…" forever.
    ///
    /// `Connecting` carries its step, so advancing through the connect
    /// sequence counts as a change and repaints too - that is what keeps
    /// the progress line live without a second dirty channel.
    pub fn phase(&self) -> PanelPhase {
        match self {
            Self::Connecting(connecting) => PanelPhase::Connecting(*connecting.progress.lock()),
            Self::Ready(_) => PanelPhase::Ready,
            Self::Failed(_) => PanelPhase::Failed,
            Self::Retired => PanelPhase::Retired,
        }
    }

    /// The adapter's process id, once there is an adapter to name.
    ///
    /// `Connecting` is the mid-restart case and `Failed` never spawned
    /// anything that survived; both answer `None`, which is the spec's "a
    /// stopped agent has no session root".
    pub fn process_id(&self) -> Option<u32> {
        match self {
            Self::Ready(handle) => handle.process_id(),
            Self::Connecting(_) | Self::Failed(_) | Self::Retired => None,
        }
    }

    /// Which agent this slot's session is running, for the header's info
    /// button and the Agents menu's Agent Info. Only a `Ready` slot has an
    /// `initialize` response to read.
    pub(crate) fn agent_identity(&self) -> AgentIdentity {
        match self {
            Self::Ready(handle) => AgentIdentity::from_info(handle.agent_info()),
            Self::Connecting(_) | Self::Failed(_) | Self::Retired => AgentIdentity::NotConnected,
        }
    }

    /// The Session Config Options the agent currently declares, which is
    /// what the panel's control bar draws its selectors from. Empty until
    /// the slot is `Ready`: nothing has been declared before the session
    /// opens.
    pub fn config_options(&self) -> Vec<ConfigOption> {
        match self {
            Self::Ready(handle) => handle.state().lock().config_options.clone(),
            Self::Connecting(_) | Self::Failed(_) | Self::Retired => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPhase {
    Connecting(ConnectStep),
    Ready,
    Failed,
    /// Torn down; never in a window's map, so never drawn.
    Retired,
}

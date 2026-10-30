//! The sidebar agent row's render inputs - what a row is built from, and how
//! big its detail lines are.
//!
//! A snapshot type, not state: `render` takes one of these out of the store
//! while the lock is held and the row closures then need neither.

use uuid::Uuid;

/// One sidebar agent row's render inputs, snapshotted out of the store
/// while its lock is held so the row closures don't need it. A struct
/// rather than the tuple this used to be, per the repo convention against
/// wide positional parameter lists.
pub(super) struct AgentRow {
    pub(super) id:            Uuid,
    pub(super) avatar:        String,
    pub(super) name:          String,
    pub(super) folder:        String,
    pub(super) state:         knot_agents::AgentState,
    /// When the agent last went idle, for its status dot's tooltip.
    pub(super) idle_since:    Option<std::time::SystemTime>,
    /// Why the agent is waiting on the user, for the same tooltip; see
    /// `knot_agents::Agent::input_reason`.
    pub(super) input_reason:  Option<String>,
    /// The usage limit that stopped it, for the same tooltip (#318).
    pub(super) session_limit: Option<knot_agents::SessionLimit>,
    pub(super) is_shell:      bool,
    pub(super) is_companion:  bool,
    /// Locked against agent messaging (#310), shown as its own detail line.
    pub(super) is_locked:     bool,
    /// Autopilot reads this agent's replies (#323), shown as a detail line.
    pub(super) autopilot:     bool,
    pub(super) header_title:  String,
    pub(super) persona_name:  Option<String>,
    /// The agent's coding-agent type (`claude`, `opencode`, ...), shown
    /// on the row so a one-letter avatar isn't the only clue to which
    /// agent is running there.
    pub(super) agent_type:    String,
    /// Whether the agent has a session. Keyed on liveness, not on
    /// activation mode: a `passive` agent that never started and a
    /// deactivated `active` one are in the same position - nothing is
    /// there - and the user needs to know which agents are live, not why
    /// each one is not.
    pub(super) is_running:    bool,
}

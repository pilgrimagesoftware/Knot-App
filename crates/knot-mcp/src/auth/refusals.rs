//! Requests the server refused, counted by who they appear to come from
//! (D10).
//!
//! A launched adapter that drops the `Authorization` header loses Knot's
//! tools to 401s rather than failing loudly. Counting refusals against the
//! agent its URL names (a non-secret hint) is what lets that agent's panel
//! say so. A refusal with no hint is counted against its `User-Agent`,
//! trimmed, and never against anything it presented.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use uuid::Uuid;

use crate::consts;

/// Who a refused request is counted against.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefusalSource {
    /// The agent its URL's `?agent=` named.
    Agent(Uuid),
    /// Its `User-Agent`, cut to [`consts::REFUSAL_USER_AGENT_MAX`]
    /// characters.
    UserAgent(String),
    /// Neither a hint nor a `User-Agent`.
    Unknown,
    /// Past [`consts::REFUSAL_SOURCES_MAX`] distinct `User-Agent`s.
    Other,
}

impl RefusalSource {
    /// The source for a refused request carrying `hint` and `user_agent`.
    ///
    /// A `User-Agent` that contains anything shaped like a Knot credential
    /// is not kept: a client that put its token there would otherwise have
    /// it shown on the MCP tab.
    pub fn of(hint: Option<Uuid>, user_agent: Option<&str>) -> Self {
        if let Some(agent) = hint {
            return Self::Agent(agent);
        }
        match user_agent.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) if !looks_like_a_credential(value) => {
                Self::UserAgent(value.chars().take(consts::REFUSAL_USER_AGENT_MAX).collect())
            }
            _ => Self::Unknown,
        }
    }
}

fn looks_like_a_credential(value: &str) -> bool {
    [consts::AGENT_TOOL_PREFIX,
     consts::AGENT_HOOK_PREFIX,
     consts::CLIENT_TOKEN_PREFIX].iter()
                                 .any(|prefix| value.contains(prefix))
}

/// What one refusal counts towards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalKind {
    /// A request a working client needs answered: a refusal here means the
    /// client is losing Knot's tools.
    Request,
    /// `GET /mcp`, the optional SSE stream. agy opens it without the
    /// header it sends on every POST, so a refused stream alone says nothing
    /// about whether the agent's tools work.
    Stream,
}

/// Refusals over the last [`consts::REFUSAL_WINDOW`], by source, and for
/// each launched agent, the refused requests since its last authenticated
/// one (D10). Cheap to clone; clones share one table, read by the MCP tab
/// and the agent panel.
#[derive(Clone, Default)]
pub struct RefusalLog {
    table: Arc<Mutex<Table>>,
}

#[derive(Default)]
struct Table {
    /// When each refusal happened, oldest first, per source; pruned to the
    /// window and capped at [`consts::REFUSAL_EVENTS_MAX`] per source.
    events:   BTreeMap<RefusalSource, VecDeque<Instant>>,
    /// Refused [`RefusalKind::Request`]s per agent since that agent last
    /// authenticated.
    since_ok: BTreeMap<Uuid, u64>,
}

impl Table {
    fn prune(&mut self, now: Instant) {
        for events in self.events.values_mut() {
            while events.front()
                        .is_some_and(|at| now.duration_since(*at) > consts::REFUSAL_WINDOW)
            {
                events.pop_front();
            }
        }
        self.events.retain(|_, events| !events.is_empty());
    }
}

impl RefusalLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one refusal of `kind` against `source`, now.
    pub fn record(&self, source: RefusalSource, kind: RefusalKind) {
        self.record_at(source, kind, Instant::now());
    }

    fn record_at(&self, source: RefusalSource, kind: RefusalKind, now: Instant) {
        let mut table = self.table.lock();
        table.prune(now);
        let user_agents = table.events
                               .keys()
                               .filter(|key| matches!(key, RefusalSource::UserAgent(_)))
                               .count();
        let source = match source {
            RefusalSource::UserAgent(_)
                if !table.events.contains_key(&source)
                   && user_agents >= consts::REFUSAL_SOURCES_MAX =>
            {
                RefusalSource::Other
            }
            source => source,
        };
        if let (RefusalSource::Agent(agent), RefusalKind::Request) = (&source, kind) {
            *table.since_ok.entry(*agent).or_default() += 1;
        }
        let events = table.events.entry(source).or_default();
        if events.len() >= consts::REFUSAL_EVENTS_MAX {
            events.pop_front();
        }
        events.push_back(now);
    }

    /// `agent` just authenticated: its earlier refusals no longer say its
    /// tools are unavailable.
    pub fn authenticated(&self, agent: Uuid) {
        self.table.lock().since_ok.remove(&agent);
    }

    /// Forgets `agent`'s run of refusals: its credentials were revoked, so a
    /// refusal from its old adapter says nothing about the next one.
    pub fn forget(&self, agent: Uuid) {
        self.table.lock().since_ok.remove(&agent);
    }

    /// Whether `agent`'s requests have been refused since it last
    /// authenticated - its adapter is not presenting the credential Knot
    /// gave it, and its panel says so (D10).
    pub fn tools_unavailable(&self, agent: Uuid) -> bool {
        self.table
            .lock()
            .since_ok
            .get(&agent)
            .is_some_and(|count| *count > 0)
    }

    /// How many requests naming `agent` were refused in the last hour.
    pub fn refused_for(&self, agent: Uuid) -> u64 {
        self.snapshot()
            .into_iter()
            .find(|(source, _)| *source == RefusalSource::Agent(agent))
            .map_or(0, |(_, count)| count)
    }

    /// Every source refused in the last hour and how often, in a stable
    /// order.
    pub fn snapshot(&self) -> Vec<(RefusalSource, u64)> {
        self.snapshot_at(Instant::now())
    }

    fn snapshot_at(&self, now: Instant) -> Vec<(RefusalSource, u64)> {
        let mut table = self.table.lock();
        table.prune(now);
        table.events
             .iter()
             .map(|(source, events)| (source.clone(), events.len() as u64))
             .collect()
    }

    /// Every refusal in the last hour, whoever it was counted against.
    pub fn total(&self) -> u64 {
        self.snapshot().iter().map(|(_, count)| count).sum()
    }
}

#[cfg(test)]
impl RefusalLog {
    /// [`Self::record`] at `at`, for the window tests.
    pub(super) fn record_for_test(&self, source: RefusalSource, kind: RefusalKind, at: Instant) {
        self.record_at(source, kind, at);
    }

    /// [`Self::snapshot`] at `at`.
    pub(super) fn snapshot_for_test(&self, at: Instant) -> Vec<(RefusalSource, u64)> {
        self.snapshot_at(at)
    }
}

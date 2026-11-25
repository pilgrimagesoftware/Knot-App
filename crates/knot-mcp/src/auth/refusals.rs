//! Requests the server refused, counted by who they appear to come from
//! (D10).
//!
//! A launched adapter that drops the `Authorization` header loses Knot's
//! tools to 401s rather than failing loudly. Counting refusals against the
//! agent its URL names (a non-secret hint) is what lets that agent's panel
//! say so. A refusal with no hint is counted against its `User-Agent`,
//! trimmed, and never against anything it presented.

use std::collections::BTreeMap;
use std::sync::Arc;

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

/// The refusal counts since Knot started. Cheap to clone; clones share one
/// table, read by the MCP tab and the agent panel.
#[derive(Clone, Default)]
pub struct RefusalLog {
    counts: Arc<Mutex<BTreeMap<RefusalSource, u64>>>,
}

impl RefusalLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one refusal against `source`.
    pub fn record(&self, source: RefusalSource) {
        let mut counts = self.counts.lock();
        let user_agents = counts.keys()
                                .filter(|key| matches!(key, RefusalSource::UserAgent(_)))
                                .count();
        // Agents are counted only while they hold a pair (the caller filters
        // the hint), which bounds them by the agents running; the cap is the
        // backstop if that ever stops holding.
        let agents = counts.keys()
                           .filter(|key| matches!(key, RefusalSource::Agent(_)))
                           .count();
        let source = match source {
            RefusalSource::UserAgent(_)
                if !counts.contains_key(&source) && user_agents >= consts::REFUSAL_SOURCES_MAX =>
            {
                RefusalSource::Other
            }
            RefusalSource::Agent(_)
                if !counts.contains_key(&source) && agents >= consts::REFUSAL_AGENTS_MAX =>
            {
                RefusalSource::Other
            }
            source => source,
        };
        *counts.entry(source).or_default() += 1;
    }

    /// How many requests naming `agent` were refused.
    pub fn refused_for(&self, agent: Uuid) -> u64 {
        self.counts
            .lock()
            .get(&RefusalSource::Agent(agent))
            .copied()
            .unwrap_or_default()
    }

    /// Every source and its count, in a stable order.
    pub fn snapshot(&self) -> Vec<(RefusalSource, u64)> {
        self.counts
            .lock()
            .iter()
            .map(|(source, count)| (source.clone(), *count))
            .collect()
    }

    /// Every refusal, whoever it was counted against.
    pub fn total(&self) -> u64 {
        self.counts.lock().values().sum()
    }
}

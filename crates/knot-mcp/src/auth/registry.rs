//! The credentials Knot has issued, kept as digests, and the one question
//! the server asks of them: who does this credential belong to?

use std::sync::Arc;

use parking_lot::Mutex;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use super::credential::{CredentialKind, Digest, Secret, digest};
use super::principal::Principal;

/// A launched agent's two credentials (D1). Handed to the launch path once;
/// the registry keeps only their digests.
#[derive(Debug, Clone)]
pub struct AgentCredentials {
    /// Goes in the MCP `Authorization` header and nowhere else (D3).
    pub tool:  Secret,
    /// Goes in the adapter's environment, for its hook scripts (D3).
    pub hooks: Secret,
}

/// Every live credential, as a digest and the principal it proves.
///
/// Cheap to clone: clones share one table, so the launch path that issues a
/// credential and the server that checks it see the same set. A handful of
/// entries at most - two per running agent and the client token - so a scan
/// of all of them per request costs nothing, and lets every comparison run
/// in constant time with no map keyed by the secret (D2).
#[derive(Clone, Default)]
pub struct CredentialRegistry {
    entries: Arc<Mutex<Vec<(Digest, Principal)>>>,
}

impl CredentialRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Issues `agent`'s tool and hook credentials, revoking any it held, in
    /// one operation under the registry's lock (D8).
    ///
    /// No request can find both the old and the new pair valid, or neither:
    /// [`Self::authenticate`] takes the same lock. Called before every
    /// adapter launch, so a restart, Restart with New Conversation and a
    /// resumed session each run on a fresh pair, and a request from the
    /// adapter that is still shutting down is refused.
    pub fn issue_agent_pair(&self, agent: Uuid) -> AgentCredentials {
        let pair = AgentCredentials { tool:  Secret::generate(CredentialKind::AgentTool),
                                      hooks: Secret::generate(CredentialKind::AgentHooks), };
        let mut entries = self.entries.lock();
        entries.retain(|(_, principal)| principal.agent() != Some(agent));
        entries.push((pair.tool.digest(), Principal::Agent(agent)));
        entries.push((pair.hooks.digest(), Principal::AgentHooks(agent)));
        pair
    }

    /// Revokes `agent`'s credentials: its adapter stopped, or the agent was
    /// closed. A no-op for an agent with none.
    pub fn revoke_agent(&self, agent: Uuid) {
        self.entries
            .lock()
            .retain(|(_, principal)| principal.agent() != Some(agent));
    }

    /// Makes `token` the one client token, revoking the previous one in the
    /// same operation - which is what regenerating it means (D1).
    pub fn set_client_token(&self, token: &Secret) {
        let mut entries = self.entries.lock();
        entries.retain(|(_, principal)| *principal != Principal::External);
        entries.push((token.digest(), Principal::External));
    }

    /// The principal `presented` proves, or `None` for a value Knot did not
    /// issue or has revoked.
    ///
    /// The presented value is hashed and its digest compared with every
    /// stored digest in constant time, without stopping at a match, so the
    /// time taken does not depend on which entry matched or how much of one
    /// did.
    pub fn authenticate(&self, presented: &str) -> Option<Principal> {
        let presented = digest(presented);
        let entries = self.entries.lock();
        let mut found = None;
        for (stored, principal) in entries.iter() {
            if bool::from(stored.ct_eq(&presented)) {
                found = Some(*principal);
            }
        }
        found
    }

    /// Whether `agent` currently holds a credential pair: it has a launched
    /// adapter. An agent that does is never acted for by the client token's
    /// holder (D5, D6).
    pub fn has_agent_pair(&self, agent: Uuid) -> bool {
        self.entries
            .lock()
            .iter()
            .any(|(_, principal)| *principal == Principal::Agent(agent))
    }
}

#[cfg(test)]
impl CredentialRegistry {
    /// How many entries speak for `agent`, read under the lock: 0 or 2 at
    /// every instant, if rotation is the single operation D8 requires.
    pub(super) fn entries_for(&self, agent: Uuid) -> usize {
        self.entries
            .lock()
            .iter()
            .filter(|(_, principal)| principal.agent() == Some(agent))
            .count()
    }
}

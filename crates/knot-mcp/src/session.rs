use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use uuid::Uuid;

use crate::auth::Principal;
use crate::consts;

/// A tracked MCP session for one agent.
#[derive(Debug, Clone)]
pub struct McpSession {
    pub id:            String,
    pub agent_id:      Uuid,
    /// The principal that opened it. Only a request authenticated as the
    /// same principal may carry on with it (D5): a session id is not a
    /// credential, so presenting another's id gets a fresh session instead.
    pub owner:         Principal,
    pub created_at:    Instant,
    pub last_activity: Instant,
}

impl McpSession {
    fn new(owner: Principal) -> Self {
        let now = Instant::now();
        let agent_id = match owner {
            Principal::Agent(agent) => agent,
            Principal::AgentHooks(_) | Principal::External => Uuid::nil(),
        };
        Self { id: Uuid::new_v4().to_string(),
               agent_id,
               owner,
               created_at: now,
               last_activity: now }
    }

    /// Whether this session is an agent's own, opened with its tool
    /// credential, rather than one the client token bound by registering.
    fn held_by_credential(&self) -> bool {
        self.owner == Principal::Agent(self.agent_id)
    }
}

#[derive(Default)]
struct SessionTable {
    sessions:         HashMap<String, McpSession>,
    agent_to_session: HashMap<Uuid, String>,
}

/// One session per agent, keyed by an opaque id. Creating a session for an
/// agent that already has one replaces the old session.
#[derive(Clone, Default)]
pub struct McpSessionManager {
    table: Arc<Mutex<SessionTable>>,
}

impl McpSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens a session for `owner`. An agent's tool credential opens one
    /// bound to that agent, replacing any session the agent held (D5): its
    /// restarted adapter takes over. The client token opens one bound to no
    /// agent (`Uuid::nil()`), exempt from the one-session-per-agent dedup
    /// below - otherwise every unbound client's handshake would evict every
    /// other's, since they would all share the nil key.
    pub fn create_session(&self, owner: Principal) -> McpSession {
        let mut table = self.table.lock();
        let session = McpSession::new(owner);
        let agent_id = session.agent_id;
        if agent_id != Uuid::nil() {
            if let Some(old_id) = table.agent_to_session.remove(&agent_id) {
                table.sessions.remove(&old_id);
            }
            table.agent_to_session.insert(agent_id, session.id.clone());
        }
        table.sessions.insert(session.id.clone(), session.clone());
        session
    }

    pub fn session(&self, id: &str) -> Option<McpSession> {
        self.table.lock().sessions.get(id).cloned()
    }

    /// Session `id`, if `principal` opened it; `None` for an unknown session
    /// or one that belongs to another principal.
    pub fn session_for(&self, id: &str, principal: Principal) -> Option<McpSession> {
        self.session(id)
            .filter(|session| session.owner == principal)
    }

    /// Whether a live session opened with `agent_id`'s tool credential holds
    /// it. Such an agent is never acted for by the client token's holder,
    /// on `/mcp` or on the hook routes (D5, D6).
    pub fn held_by_credential(&self, agent_id: Uuid) -> bool {
        let table = self.table.lock();
        table.agent_to_session
             .get(&agent_id)
             .and_then(|id| table.sessions.get(id))
             .is_some_and(McpSession::held_by_credential)
    }

    /// How many sessions are live right now - what the heartbeat reports, so
    /// an idle server can be told from one nothing is connected to.
    pub fn len(&self) -> usize {
        self.table.lock().sessions.len()
    }

    /// Whether no session is live. Paired with [`Self::len`] because clippy
    /// asks for it; the heartbeat uses the count.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn session_for_agent(&self, agent_id: Uuid) -> Option<McpSession> {
        let table = self.table.lock();
        let id = table.agent_to_session.get(&agent_id)?;
        table.sessions.get(id).cloned()
    }

    /// Binds session `id` to `agent_id`, as creating it for that agent
    /// would have: a session the agent held before is dropped, and the
    /// session stops speaking for any agent it was bound to. Does nothing
    /// for an unknown session, and nothing when a session the client token
    /// opened would displace one the agent's own credential holds (D5).
    pub fn bind(&self, id: &str, agent_id: Uuid) {
        let mut table = self.table.lock();
        let Some((previous, owner)) = table.sessions
                                           .get(id)
                                           .map(|session| (session.agent_id, session.owner))
        else {
            return;
        };
        let displaces_credential = table.agent_to_session
                                        .get(&agent_id)
                                        .filter(|holder| holder.as_str() != id)
                                        .and_then(|holder| table.sessions.get(holder))
                                        .is_some_and(McpSession::held_by_credential);
        if displaces_credential && owner != Principal::Agent(agent_id) {
            return;
        }
        if previous != Uuid::nil()
           && table.agent_to_session.get(&previous).map(String::as_str) == Some(id)
        {
            table.agent_to_session.remove(&previous);
        }
        if let Some(old_id) = table.agent_to_session.insert(agent_id, id.to_owned())
           && old_id != id
        {
            table.sessions.remove(&old_id);
        }
        if let Some(session) = table.sessions.get_mut(id) {
            session.agent_id = agent_id;
        }
    }

    /// Whether a live session other than `id` is bound to `agent_id`.
    pub fn held_elsewhere(&self, agent_id: Uuid, id: Option<&str>) -> bool {
        let table = self.table.lock();
        table.agent_to_session
             .get(&agent_id)
             .is_some_and(|holder| {
                 Some(holder.as_str()) != id && table.sessions.contains_key(holder)
             })
    }

    pub fn touch(&self, id: &str) {
        if let Some(session) = self.table.lock().sessions.get_mut(id) {
            session.last_activity = Instant::now();
        }
    }

    pub fn remove(&self, id: &str) {
        let mut table = self.table.lock();
        if let Some(session) = table.sessions.remove(id) {
            table.agent_to_session.remove(&session.agent_id);
        }
    }

    pub fn remove_for_agent(&self, agent_id: Uuid) {
        let mut table = self.table.lock();
        if let Some(id) = table.agent_to_session.remove(&agent_id) {
            table.sessions.remove(&id);
        }
    }

    /// Removes sessions idle longer than the crate's default timeout (1 hour).
    pub fn cleanup_stale_default(&self) {
        self.cleanup_stale(consts::DEFAULT_SESSION_TIMEOUT);
    }

    /// Removes sessions whose `last_activity` is older than `timeout`.
    pub fn cleanup_stale(&self, timeout: Duration) {
        let mut table = self.table.lock();
        let stale: Vec<String> = table.sessions
                                      .values()
                                      .filter(|s| s.last_activity.elapsed() > timeout)
                                      .map(|s| s.id.clone())
                                      .collect();
        for id in stale {
            if let Some(session) = table.sessions.remove(&id) {
                table.agent_to_session.remove(&session.agent_id);
            }
        }
    }
}

#[cfg(test)]
mod tests;

//! A server's tools, listed on demand when its row is opened (#40).
//!
//! Contract: "A server's tools can be listed on demand" in
//! `openspec/specs/agent-mcp-status/spec.md`.
//!
//! What a row may be listed from is decided before anything starts, by
//! [`source_of`]: Knot's own tools come straight from its catalog, and
//! another server is asked with `knot-mcp-client` only when the probe saw it
//! connected (or could not tell) and, for a stdio server, only when the
//! listing gave the exact command to start it. Everything else says why its
//! tools cannot be listed instead of guessing.
//!
//! Listings are as old as the probe that produced their rows: a new
//! inventory clears them, along with which rows were open.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use knot_mcp_client::{Endpoint, Tool};
use knot_mcp_probe::{ServerState, Target};
use parking_lot::Mutex;
use uuid::Uuid;

use super::rows::SectionRow;
use crate::workspace_window::WorkspaceWindow;

/// Which row a listing belongs to. Built from everything that identifies the
/// server, since names come from foreign output and are not assumed unique.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ToolsKey(String);

impl ToolsKey {
    pub(crate) fn of(row: &SectionRow) -> Self {
        match row {
            SectionRow::Knot(_) => Self("knot".to_owned()),
            SectionRow::Agent(server) => Self(format!("agent\u{1f}{}\u{1f}{}\u{1f}{}",
                                                      server.target.token(),
                                                      server.name,
                                                      server.target.full())),
        }
    }
}

/// Why a row's tools are not listed at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unlisted {
    /// The server is disabled, awaiting approval, needs sign-in or failed:
    /// the agent is not connected to it, so neither is Knot.
    NotConnected,
    /// A stdio server whose listing gave its command only as text, which
    /// cannot be split back into a program and its arguments safely.
    NoLaunch,
}

/// One row's tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolsState {
    Listing,
    Listed(Vec<Tool>),
    Failed(String),
    Unlisted(Unlisted),
}

/// Where a row's tools come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolSource {
    /// Knot's own catalog, in process.
    Knot,
    /// Another server, asked over MCP.
    Remote(Endpoint),
    Unlisted(Unlisted),
}

/// Whether a server in `state` is one Knot may connect to for its tools.
///
/// Every variant named, so a new state fails to compile here until someone
/// decides whether connecting to it is acceptable.
fn connects(state: ServerState) -> bool {
    match state {
        ServerState::Connected | ServerState::Unknown => true,
        ServerState::NeedsAuthentication
        | ServerState::PendingApproval
        | ServerState::Disabled
        | ServerState::Failed => false,
    }
}

/// What `row`'s tools may be listed from. Knot's own are listed while its
/// server is running, since only then can the agent call them. A stdio
/// server with no directory of its own starts in the agent's folder, as the
/// agent would start it.
pub(crate) fn source_of(row: &SectionRow, agent_folder: &Path) -> ToolSource {
    let server = match row {
        SectionRow::Knot(knot) if matches!(knot.state, knot_mcp::ServerState::Running { .. }) => {
            return ToolSource::Knot;
        }
        SectionRow::Knot(_) => return ToolSource::Unlisted(Unlisted::NotConnected),
        SectionRow::Agent(server) => server,
    };
    if !connects(server.state) {
        return ToolSource::Unlisted(Unlisted::NotConnected);
    }
    match &server.target {
        Target::Http { url } => ToolSource::Remote(Endpoint::Http { url: url.clone() }),
        Target::Stdio { .. } => {
            let Some(launch) = &server.launch
            else {
                return ToolSource::Unlisted(Unlisted::NoLaunch);
            };
            let mut launch = launch.clone();
            launch.cwd.get_or_insert_with(|| agent_folder.to_path_buf());
            ToolSource::Remote(Endpoint::Stdio(launch))
        }
    }
}

/// Knot's own tools, as a row lists them.
pub(crate) fn knot_tools() -> Vec<Tool> {
    knot_mcp_tools::tool_catalog().into_iter()
                                  .map(|tool| Tool { name:        tool.name,
                                                     description: Some(tool.description), })
                                  .collect()
}

/// Rows with a listing in flight.
pub(crate) type ToolsInFlight = Arc<Mutex<BTreeSet<(Uuid, ToolsKey)>>>;

/// Where a finished listing reports, and the main thread drains.
pub(crate) type ToolsResults = Arc<Mutex<Vec<(Uuid, ToolsKey, Result<Vec<Tool>, String>)>>>;

/// A claim on one row's listing, released when it drops - and reporting a
/// failure itself if the task unwinds, so a row cannot sit on "listing"
/// with nothing running. The same discipline as `ProbeClaim`.
pub(crate) struct ToolsClaim {
    agent:     Uuid,
    key:       ToolsKey,
    in_flight: ToolsInFlight,
    results:   ToolsResults,
    reported:  bool,
}

impl ToolsClaim {
    /// Claims this row's listing, or `None` when one is already running.
    pub(crate) fn claim(in_flight: &ToolsInFlight, results: &ToolsResults, agent: Uuid,
                        key: ToolsKey)
                        -> Option<Self> {
        in_flight.lock()
                 .insert((agent, key.clone()))
                 .then(|| Self { agent,
                                 key,
                                 in_flight: Arc::clone(in_flight),
                                 results: Arc::clone(results),
                                 reported: false })
    }

    pub(crate) fn report(mut self, outcome: Result<Vec<Tool>, String>) {
        self.results
            .lock()
            .push((self.agent, self.key.clone(), outcome));
        self.reported = true;
    }
}

impl Drop for ToolsClaim {
    fn drop(&mut self) {
        if !self.reported {
            self.results
                .lock()
                .push((self.agent, self.key.clone(), Err("the listing did not finish".to_owned())));
        }
        self.in_flight
            .lock()
            .remove(&(self.agent, self.key.clone()));
    }
}

impl WorkspaceWindow {
    /// Opens or shuts `row`'s tools, starting a listing when it opens with
    /// none - or after one failed, so opening it again is how to retry.
    pub(in crate::workspace_window) fn toggle_mcp_tools(&mut self, agent_id: Uuid,
                                                        row: &SectionRow) {
        let key = ToolsKey::of(row);
        let section = self.mcp_sections.entry(agent_id).or_default();
        if !section.toggle_tools(&key) {
            return;
        }
        if matches!(section.tools(&key),
                    Some(ToolsState::Listed(_) | ToolsState::Listing))
        {
            return;
        }
        let Some(folder) = self.store
                               .lock()
                               .agent(agent_id)
                               .map(|agent| std::path::PathBuf::from(&agent.folder))
        else {
            return;
        };
        let state = match source_of(row, &folder) {
            ToolSource::Knot => ToolsState::Listed(knot_tools()),
            ToolSource::Unlisted(why) => ToolsState::Unlisted(why),
            ToolSource::Remote(endpoint) => {
                let Some(claim) = ToolsClaim::claim(&self.mcp_tools_in_flight,
                                                    &self.mcp_tool_results,
                                                    agent_id,
                                                    key.clone())
                else {
                    return;
                };
                let path = knot_core::exec_path::search_path();
                // `spawn_blocking`: a stdio server is a process, and HTTP
                // here is a blocking client. The claim goes into the task on
                // the next line, so nothing can drop it unreported.
                self.runtime.spawn_blocking(move || {
                                let listed =
                                    knot_mcp_client::list_tools(&endpoint,
                                                                &path,
                                                                knot_mcp_client::consts::LIST_TIMEOUT);
                                claim.report(listed.map_err(|error| error.to_string()));
                            });
                ToolsState::Listing
            }
        };
        if let Some(section) = self.mcp_sections.get_mut(&agent_id) {
            section.set_tools(key, state);
        }
    }

    /// Moves finished listings into their rows, answering whether any
    /// arrived. The only drain of `mcp_tool_results`.
    ///
    /// A listing for an agent whose section has gone is dropped rather than
    /// recreating the section, so a removed agent leaves nothing behind.
    pub(in crate::workspace_window) fn drain_mcp_tools(&mut self) -> bool {
        let finished = std::mem::take(&mut *self.mcp_tool_results.lock());
        if finished.is_empty() {
            return false;
        }
        for (agent_id, key, outcome) in finished {
            if let Some(section) = self.mcp_sections.get_mut(&agent_id) {
                section.set_tools(key, match outcome {
                           Ok(tools) => ToolsState::Listed(tools),
                           Err(error) => ToolsState::Failed(error),
                       });
            }
        }
        true
    }
}

#[cfg(test)]
mod tests;

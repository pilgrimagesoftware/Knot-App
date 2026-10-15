//! `codex mcp list --json`.
//!
//! Codex's listing is configuration, not a probe: unlike Claude Code and
//! Gemini, which each test a server's connection as they list it, Codex only
//! reports what is configured and whether the user disabled it. There is no
//! field here saying a server answered - `auth_status` names the transport's
//! declared auth mechanism (`"unknown"`, `"unsupported"`, `"o_auth"`), not
//! whether that auth has already succeeded. Claiming [`ServerState::Connected`]
//! or [`ServerState::NeedsAuthentication`] from that would be a confident
//! wrong answer, so an enabled row reads as [`ServerState::Unknown`] - the
//! variant for "there is a server here and I cannot tell you about it" - and
//! only `enabled: false` becomes a classified state.
//!
//! ```json
//! [
//!   { "name": "akeyless", "enabled": true, "disabled_reason": null,
//!     "transport": { "type": "streamable_http", "url": "http://127.0.0.1:8086/mcp" } },
//!   { "name": "codex_app", "enabled": false, "disabled_reason": null,
//!     "transport": { "type": "stdio", "command": "/path/to/launch", "args": ["./server.mjs"] } }
//! ]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use knot_mcp_client::StdioServer;
use serde::Deserialize;

use crate::server::{ServerRow, Target};
use crate::state::ServerState;

#[derive(Deserialize)]
struct Entry {
    name:            String,
    enabled:         bool,
    disabled_reason: Option<String>,
    transport:       TransportJson,
}

/// Only the fields a target needs. `type` is deliberately not read: the
/// target is derived from which of `url`/`command` is present, not from
/// Codex's name for the transport, so an unrecognized future `type` still
/// resolves correctly instead of falling through to "cannot determine".
#[derive(Deserialize)]
struct TransportJson {
    command: Option<String>,
    #[serde(default)]
    args:    Vec<String>,
    url:     Option<String>,
    /// Variables set for the server. `null` when there are none.
    #[serde(default)]
    env:     Option<BTreeMap<String, String>>,
    cwd:     Option<PathBuf>,
}

/// Reads a listing, or `None` when it is not JSON this shape recognizes.
///
/// `None` rather than an empty `Vec` for anything that fails to parse: an
/// agent's listing having moved away from this shape is "could not read
/// this", never "no servers configured".
pub(crate) fn parse(output: &str) -> Option<Vec<ServerRow>> {
    let entries: Vec<Entry> = serde_json::from_str(output.trim()).ok()?;

    Some(entries.into_iter().map(row_from).collect())
}

fn row_from(entry: Entry) -> ServerRow {
    let target = target_of(&entry.transport);
    let state = if entry.enabled {
        ServerState::Unknown
    }
    else {
        ServerState::Disabled
    };

    let launch = launch_of(&entry.transport);
    let row = ServerRow::new(entry.name, target, state);
    let row = match launch {
        Some(launch) => row.with_launch(launch),
        None => row,
    };

    match entry.disabled_reason {
        Some(reason) => row.with_detail(reason),
        None => row,
    }
}

/// A target from the transport's own shape, rather than guessed from a
/// `type` this crate has not seen: a `url` is an HTTP target and a `command`
/// is a stdio one regardless of what future transport kinds Codex adds.
fn target_of(transport: &TransportJson) -> Target {
    if let Some(url) = &transport.url {
        return Target::Http { url: url.clone() };
    }

    let command = transport.command.clone().unwrap_or_default();
    let full = if transport.args.is_empty() {
        command
    }
    else {
        format!("{command} {}", transport.args.join(" "))
    };

    Target::Stdio { command: full }
}

/// How a stdio server is started, from the separate words Codex lists it
/// in. Codex is the one listing that gives them, which is why it is the one
/// whose stdio servers' tools can be listed (#40).
fn launch_of(transport: &TransportJson) -> Option<StdioServer> {
    if transport.url.is_some() {
        return None;
    }
    let program = transport.command
                           .clone()
                           .filter(|command| !command.is_empty())?;
    Some(StdioServer { program,
                       args: transport.args.clone(),
                       env: transport.env
                                     .clone()
                                     .unwrap_or_default()
                                     .into_iter()
                                     .collect(),
                       cwd: transport.cwd.clone() })
}

#[cfg(test)]
mod tests;

//! One tool, and the entry point that lists them.

use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::endpoint::Endpoint;
use crate::error::Result;
use crate::http::HttpExchange;
use crate::session::list_tools_over;
use crate::stdio::StdioExchange;

/// A tool as a row lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tool {
    pub name:        String,
    #[serde(default)]
    pub description: Option<String>,
}

/// The tools `endpoint` provides, in the order it lists them, at most
/// [`crate::consts::MAX_TOOLS`].
///
/// `path` is the `PATH` a stdio server is started with - the same one an
/// agent's own launch uses, so `npx` and `uvx` resolve. A stdio server is
/// stopped, with everything it started, before this returns.
///
/// # Errors
///
/// [`crate::ClientError`] for a server that cannot be started or reached,
/// does not answer within `timeout`, wants a credential, or answers in a
/// shape this client does not read.
pub fn list_tools(endpoint: &Endpoint, path: &str, timeout: Duration) -> Result<Vec<Tool>> {
    let deadline = Instant::now() + timeout;
    match endpoint {
        Endpoint::Http { url } => list_tools_over(&mut HttpExchange::new(url, deadline)),
        Endpoint::Stdio(server) => {
            list_tools_over(&mut StdioExchange::spawn(server, path, deadline)?)
        }
    }
}

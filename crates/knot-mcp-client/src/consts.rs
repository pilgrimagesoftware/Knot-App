//! Bounds on one listing.

use std::time::Duration;

/// The protocol revision offered in `initialize`. A server answering with an
/// older one it supports is accepted: `tools/list` has the same shape in
/// every revision this client reads.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// How this client names itself in `initialize`.
pub const CLIENT_NAME: &str = "knot";

/// How long one listing may take, start to finish. A stdio server started
/// through `npx` or `uvx` may download itself first, so this is generous;
/// it exists so a server that never answers cannot hold a row on
/// "listing" forever.
pub const LIST_TIMEOUT: Duration = Duration::from_secs(20);

/// The most tools kept from one server. A row's list is drawn whole, so an
/// unbounded answer would be an unbounded row.
pub const MAX_TOOLS: usize = 500;

/// The most `tools/list` pages followed. A server whose cursor never ends
/// stops here rather than looping until the timeout.
pub const MAX_PAGES: usize = 20;

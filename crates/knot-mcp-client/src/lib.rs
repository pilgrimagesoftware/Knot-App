//! Asks an MCP server which tools it provides: `initialize`, then
//! `tools/list`, and nothing else.
//!
//! Contract: "A server's tools can be listed on demand" in
//! `openspec/specs/agent-mcp-status/spec.md`.
//!
//! The fourth crate in the MCP family, and the names say which is which:
//! `knot-mcp` *serves* Knot's own server, `knot-mcp-tools` *catalogs* that
//! server's tools, `knot-mcp-probe` *interrogates* an agent's CLI about the
//! servers it has, and this one *connects* to one of those servers. It calls
//! no tool, holds no credential, and keeps no connection: every listing is a
//! fresh session that ends when the answer is read.
//!
//! Runtime-agnostic, like `knot-mcp-probe`: it blocks, so wrap calls in
//! `spawn_blocking` at an async boundary.

pub mod consts;
pub mod endpoint;
pub mod error;
mod http;
mod session;
mod stdio;
pub mod tool;

pub use endpoint::{Endpoint, StdioServer};
pub use error::{ClientError, Result};
pub use tool::{Tool, list_tools};

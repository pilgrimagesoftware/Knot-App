//! Where a server is reached.

use std::path::PathBuf;

/// A server to list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// A Streamable HTTP endpoint, reached without credentials.
    Http { url: String },
    /// A command speaking MCP over its pipes.
    Stdio(StdioServer),
}

/// Exactly how to start a stdio server: the program, its arguments as
/// separate words, and the variables it is given on top of `PATH`.
///
/// Never parsed out of a flattened command line. A listing that only gives
/// the line as text cannot say where one argument ends and the next begins,
/// and starting the wrong command is worse than not listing its tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioServer {
    pub program: String,
    pub args:    Vec<String>,
    pub env:     Vec<(String, String)>,
    pub cwd:     Option<PathBuf>,
}

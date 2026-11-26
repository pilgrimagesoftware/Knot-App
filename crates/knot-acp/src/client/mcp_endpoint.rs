//! Knot's MCP server as one `mcpServers` entry of a session request.

use std::fmt;

use serde_json::{Value, json};

/// Where an agent reaches Knot's MCP server, and the bearer credential it
/// presents there (`openspec/changes/mcp-auth/design.md` D3).
///
/// The credential travels in the entry's `headers` - ACP's `McpServerHttp`
/// carries `HttpHeader { name, value }` pairs
/// (<https://agentclientprotocol.com/protocol/v1/schema>) - never in the
/// URL. Its `Debug` leaves the credential out, so logging a request target
/// cannot leak it.
#[derive(Clone, Copy)]
pub struct McpEndpoint<'a> {
    /// The server's URL, with the agent's `?agent=` hint.
    pub url:    &'a str,
    /// The agent's tool credential, sent as `Authorization: Bearer`.
    pub bearer: Option<&'a str>,
}

impl McpEndpoint<'_> {
    /// The entry's `headers` array: the `Authorization` header when there is
    /// a credential, and empty otherwise - ACP requires the key either way.
    pub(super) fn headers(&self) -> Value {
        match self.bearer {
            Some(credential) => {
                json!([{ "name": "Authorization", "value": format!("Bearer {credential}") }])
            }
            None => json!([]),
        }
    }
}

impl fmt::Debug for McpEndpoint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpEndpoint")
         .field("url", &self.url)
         .field("bearer", &self.bearer.map(|_| "<redacted>"))
         .finish()
    }
}

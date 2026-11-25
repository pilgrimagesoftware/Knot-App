//! The MCP credentials the app holds: the registry the server authenticates
//! against, the refusal counts the UI shows, and the client token kept in
//! the Keychain.
//!
//! Contract: `openspec/changes/mcp-auth/design.md` D1, D3, D8 and D10.
//! Agent credentials are issued just before an adapter starts
//! (`workspace_window::panel::session`) and revoked when its panel session
//! is retired, so every launch, restart and resumed session runs on a fresh
//! pair and an adapter that is shutting down is refused.

mod client_token;
mod shared;

// UNWIRED(mcp-auth task 6.1): the MCP tab's regenerate button is the
// caller; until it lands, only the tests use these.
#[allow(unused_imports)]
pub(crate) use client_token::{ClientTokenStore, regenerate};
pub(crate) use client_token::{KeychainTokenStore, load_or_create};
pub(crate) use shared::{ClientToken, McpCredentials};

#[cfg(test)]
mod tests;

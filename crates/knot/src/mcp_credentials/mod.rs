//! The MCP credentials the app holds: the registry the server authenticates
//! against, the refusal counts the UI shows, and the client token kept in
//! the Keychain (a 0600 file on other platforms).
//!
//! Contract: `openspec/changes/mcp-auth/design.md` D1, D3, D8 and D10.
//! Agent credentials are issued just before an adapter starts
//! (`workspace_window::panel::session`) and revoked when its panel session
//! is retired, so every launch, restart and resumed session runs on a fresh
//! pair and an adapter that is shutting down is refused.

mod client_token;
mod shared;

// Named here for the tests; the platform store builds its own off macOS.
#[cfg(test)]
pub(crate) use client_token::FileTokenStore;
pub(crate) use client_token::{ClientTokenStore, regenerate};
pub(crate) use client_token::{LoadedClientToken, PlatformTokenStore, load_or_create};
pub(crate) use shared::{ClientToken, McpCredentials};

#[cfg(test)]
mod tests;

//! The client token: read from the Keychain at startup, created when there
//! is none, and regenerated on request (D1, task 1.2).
//!
//! Behind [`ClientTokenStore`] so the logic is tested with an in-memory
//! store; the Keychain itself is exercised by hand on a signed build.

use knot_mcp::{CredentialKind, CredentialRegistry, Secret};

/// Where the client token persists between launches.
pub(crate) trait ClientTokenStore {
    /// The stored value, if there is one.
    fn load(&self) -> Option<String>;
    /// Replaces the stored value.
    fn save(&self, token: &Secret) -> Result<(), String>;
}

/// The token in effect: the stored one when it is a token Knot could have
/// generated, else a new one, saved. Made the registry's client token
/// either way.
///
/// A token that cannot be saved still serves this launch - the MCP tab
/// shows it - and the next launch generates another. The failure is logged
/// without the token.
pub(crate) fn load_or_create(store: &dyn ClientTokenStore, registry: &CredentialRegistry)
                             -> Secret {
    let token = match store.load()
                           .as_deref()
                           .and_then(Secret::from_stored_client_token)
    {
        Some(token) => token,
        None => fresh(store),
    };
    registry.set_client_token(&token);
    token
}

/// A new client token, saved and made the registry's, which revokes the
/// previous one in the same step.
// UNWIRED(mcp-auth task 6.1): called from the MCP tab's regenerate button.
#[allow(dead_code)]
pub(crate) fn regenerate(store: &dyn ClientTokenStore, registry: &CredentialRegistry) -> Secret {
    let token = fresh(store);
    registry.set_client_token(&token);
    token
}

fn fresh(store: &dyn ClientTokenStore) -> Secret {
    let token = Secret::generate(CredentialKind::Client);
    if let Err(error) = store.save(&token) {
        eprintln!("the MCP client token could not be saved: {error}");
    }
    token
}

/// The macOS Keychain, as a generic password under
/// [`crate::consts::MCP_CLIENT_TOKEN_SERVICE`]. Elsewhere there is no
/// Keychain, so the token lasts one launch.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct KeychainTokenStore;

#[cfg(target_os = "macos")]
impl ClientTokenStore for KeychainTokenStore {
    fn load(&self) -> Option<String> {
        use crate::consts::{MCP_CLIENT_TOKEN_ACCOUNT, MCP_CLIENT_TOKEN_SERVICE};
        let bytes =
            security_framework::passwords::get_generic_password(MCP_CLIENT_TOKEN_SERVICE,
                                                                MCP_CLIENT_TOKEN_ACCOUNT).ok()?;
        String::from_utf8(bytes).ok()
    }

    fn save(&self, token: &Secret) -> Result<(), String> {
        use crate::consts::{MCP_CLIENT_TOKEN_ACCOUNT, MCP_CLIENT_TOKEN_SERVICE};
        security_framework::passwords::set_generic_password(MCP_CLIENT_TOKEN_SERVICE,
                                                            MCP_CLIENT_TOKEN_ACCOUNT,
                                                            token.expose().as_bytes())
            .map_err(|error| error.to_string())
    }
}

#[cfg(not(target_os = "macos"))]
impl ClientTokenStore for KeychainTokenStore {
    fn load(&self) -> Option<String> {
        None
    }

    fn save(&self, _token: &Secret) -> Result<(), String> {
        Ok(())
    }
}

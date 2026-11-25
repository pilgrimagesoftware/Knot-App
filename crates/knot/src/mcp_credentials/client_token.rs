//! The client token: read from the Keychain at startup, created when there
//! is none, and regenerated on request (D1, task 1.2).
//!
//! Behind [`ClientTokenStore`] so the logic is tested with an in-memory
//! store; the Keychain itself is exercised by hand on a signed build.

use knot_mcp::{CredentialKind, CredentialRegistry, Secret};

/// Where the client token persists between launches.
pub(crate) trait ClientTokenStore {
    /// The stored value: `Ok(None)` only when there is definitely none, and
    /// `Err` when the store could not be read - access denied, a locked
    /// keychain, or an item that refuses a re-signed build.
    fn load(&self) -> Result<Option<String>, String>;
    /// Replaces the stored value.
    fn save(&self, token: &Secret) -> Result<(), String>;
}

/// The client token in effect for this launch.
#[derive(Debug, Clone)]
pub(crate) struct LoadedClientToken {
    pub(crate) token:   Secret,
    /// Why this token is not the stored one, or will not outlast this
    /// launch: the store could not be read, or the token could not be
    /// saved. For the MCP tab to show; `None` when all is well.
    pub(crate) problem: Option<String>,
}

/// The token in effect, made the registry's client token.
///
/// - The stored token, when it is one Knot could have generated.
/// - A new one, saved, when there is none or what is stored is damaged.
/// - When the store cannot be read, a token for this launch only, never saved:
///   overwriting a token that exists but could not be read would cut off every
///   client the user registered with it. The error is kept for the MCP tab, and
///   logged without the token.
pub(crate) fn load_or_create(store: &dyn ClientTokenStore, registry: &CredentialRegistry)
                             -> LoadedClientToken {
    let loaded = match store.load() {
        Ok(stored) => match stored.as_deref().and_then(Secret::from_stored_client_token) {
            Some(token) => LoadedClientToken { token,
                                               problem: None },
            None => fresh(store),
        },
        Err(error) => {
            eprintln!("the MCP client token could not be read, so this launch uses one that is \
                       not saved: {error}");
            LoadedClientToken { token:   Secret::generate(CredentialKind::Client),
                                problem: Some(error), }
        }
    };
    registry.set_client_token(&loaded.token);
    loaded
}

/// A new client token, saved and made the registry's, which revokes the
/// previous one in the same step.
pub(crate) fn regenerate(store: &dyn ClientTokenStore, registry: &CredentialRegistry)
                         -> LoadedClientToken {
    let loaded = fresh(store);
    registry.set_client_token(&loaded.token);
    loaded
}

/// A new token, saved if the store takes it.
fn fresh(store: &dyn ClientTokenStore) -> LoadedClientToken {
    let token = Secret::generate(CredentialKind::Client);
    let problem = store.save(&token).err();
    if let Some(error) = &problem {
        eprintln!("the MCP client token could not be saved: {error}");
    }
    LoadedClientToken { token, problem }
}

/// The macOS Keychain, as a generic password under
/// [`crate::consts::MCP_CLIENT_TOKEN_SERVICE`]. Elsewhere there is no
/// Keychain, so the token lasts one launch.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct KeychainTokenStore;

#[cfg(target_os = "macos")]
impl ClientTokenStore for KeychainTokenStore {
    fn load(&self) -> Result<Option<String>, String> {
        use crate::consts::{
            KEYCHAIN_ITEM_NOT_FOUND, MCP_CLIENT_TOKEN_ACCOUNT, MCP_CLIENT_TOKEN_SERVICE,
        };
        match security_framework::passwords::get_generic_password(MCP_CLIENT_TOKEN_SERVICE,
                                                                  MCP_CLIENT_TOKEN_ACCOUNT)
        {
            // Bytes that are not text are a damaged item, which
            // `load_or_create` replaces like any other damaged value.
            Ok(bytes) => Ok(Some(String::from_utf8(bytes).unwrap_or_default())),
            Err(error) if error.code() == KEYCHAIN_ITEM_NOT_FOUND => Ok(None),
            Err(error) => Err(error.to_string()),
        }
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
    fn load(&self) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn save(&self, _token: &Secret) -> Result<(), String> {
        Ok(())
    }
}

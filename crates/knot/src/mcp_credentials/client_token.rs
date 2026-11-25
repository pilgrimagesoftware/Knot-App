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

/// Where this platform keeps the client token: the macOS Keychain, as a
/// generic password under [`crate::consts::MCP_CLIENT_TOKEN_SERVICE`]; on
/// other platforms, which Knot has no secret store for, a [`FileTokenStore`]
/// at [`knot_core::mcp_client_token_path`], readable by its owner only.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PlatformTokenStore;

/// The client token in a file, written with owner-only permissions (0600).
// Off macOS it is the platform store; on macOS only its tests build it.
#[cfg_attr(target_os = "macos", allow(dead_code))]
#[derive(Debug, Clone)]
pub(crate) struct FileTokenStore {
    pub(crate) path: std::path::PathBuf,
}

impl ClientTokenStore for FileTokenStore {
    fn load(&self) -> Result<Option<String>, String> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(Some(text.trim().to_owned())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Written to a sibling with owner-only permissions from the moment it
    /// exists, then renamed over the old file, so no reader ever sees a
    /// half-written token or a world-readable one.
    fn save(&self, token: &Secret) -> Result<(), String> {
        use std::io::Write as _;
        let directory = self.path
                            .parent()
                            .ok_or_else(|| "the token file has no directory".to_owned())?;
        std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let staging = self.path.with_extension("tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&staging).map_err(|error| error.to_string())?;
        file.write_all(token.expose().as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| error.to_string())?;
        std::fs::rename(&staging, &self.path).map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
impl ClientTokenStore for PlatformTokenStore {
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
impl ClientTokenStore for PlatformTokenStore {
    fn load(&self) -> Result<Option<String>, String> {
        Self::file()?.load()
    }

    fn save(&self, token: &Secret) -> Result<(), String> {
        Self::file()?.save(token)
    }
}

#[cfg(not(target_os = "macos"))]
impl PlatformTokenStore {
    fn file() -> Result<FileTokenStore, String> {
        knot_core::mcp_client_token_path().map(|path| FileTokenStore { path })
                                          .ok_or_else(|| {
                                              "no home directory to keep the token in".to_owned()
                                          })
    }
}

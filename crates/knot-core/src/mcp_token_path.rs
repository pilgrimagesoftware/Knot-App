//! Where the MCP client token is kept on a platform with no Keychain
//! (`openspec/changes/mcp-auth/design.md` D1): a file beside the settings
//! documents, readable by its owner only. On macOS the token is in the
//! Keychain and this file is not used.

use std::path::PathBuf;

use directories::ProjectDirs;

use crate::consts::{APP_NAME, MCP_CLIENT_TOKEN_FILE, ORG_NAME, ORG_QUALIFIER};

/// The client token file in this application's configuration directory, or
/// `None` when no home directory can be resolved.
pub fn mcp_client_token_path() -> Option<PathBuf> {
    ProjectDirs::from(ORG_QUALIFIER, ORG_NAME, APP_NAME).map(|dirs| {
                                                            dirs.config_dir()
                                                                .join(MCP_CLIENT_TOKEN_FILE)
                                                        })
}

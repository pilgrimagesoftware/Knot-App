//! One credential: its kind, its value and the digest the registry keeps
//! in its place (D2).

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};

use crate::consts;

/// The three kinds of credential Knot issues (D1). The kind is the value's
/// prefix, so a leaked credential says what it is to a secret scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    /// A launched agent's tool credential: `/mcp` and the status GET.
    AgentTool,
    /// A launched agent's hook credential: the hook POSTs only.
    AgentHooks,
    /// The client token for MCP clients the user registers by hand.
    Client,
}

impl CredentialKind {
    /// The prefix every value of this kind starts with.
    pub fn prefix(self) -> &'static str {
        match self {
            Self::AgentTool => consts::AGENT_TOOL_PREFIX,
            Self::AgentHooks => consts::AGENT_HOOK_PREFIX,
            Self::Client => consts::CLIENT_TOKEN_PREFIX,
        }
    }
}

/// A credential's value.
///
/// Deliberately neither `Display` nor a `Debug` that shows it: the only way
/// to the text is [`Secret::expose`], so every place a credential leaves
/// Knot (a header, an environment variable, the Keychain, the clipboard) is
/// a greppable call.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// A new credential of `kind`: [`consts::CREDENTIAL_BYTES`] bytes from
    /// the operating system's CSPRNG, base64url without padding, after the
    /// kind's prefix.
    ///
    /// # Panics
    ///
    /// When the operating system cannot supply random bytes. There is no
    /// safe fallback: a credential from a weaker source would be worse than
    /// none, and a machine without a working random source cannot run Knot's
    /// other cryptography either.
    pub fn generate(kind: CredentialKind) -> Self {
        let mut bytes = [0_u8; consts::CREDENTIAL_BYTES];
        getrandom::fill(&mut bytes).expect("the operating system's random source is available");
        Self(format!("{}{}", kind.prefix(), URL_SAFE_NO_PAD.encode(bytes)))
    }

    /// A client token read back from storage, or `None` when `value` is not
    /// one Knot could have generated - so a damaged Keychain item is
    /// replaced rather than trusted.
    pub fn from_stored_client_token(value: &str) -> Option<Self> {
        let encoded = value.strip_prefix(consts::CLIENT_TOKEN_PREFIX)?;
        let decoded = URL_SAFE_NO_PAD.decode(encoded).ok()?;
        (decoded.len() == consts::CREDENTIAL_BYTES).then(|| Self(value.to_owned()))
    }

    /// The credential's text, for the one place it is being handed to.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// The digest the registry stores instead of the value.
    pub(super) fn digest(&self) -> Digest {
        digest(&self.0)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

/// A SHA-256 digest of a credential's text.
pub(super) type Digest = [u8; 32];

/// The digest of `value`, whether or not it is a credential Knot issued.
pub(super) fn digest(value: &str) -> Digest {
    Sha256::digest(value.as_bytes()).into()
}

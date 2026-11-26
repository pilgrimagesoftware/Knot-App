//! Credentials for the HTTP suites (task 3.3): one client token per test
//! binary, a registry that accepts it, and a client that presents it.

#![allow(dead_code)] // Each test binary uses a different part of this.

use std::sync::LazyLock;

use knot_mcp::{CredentialKind, CredentialRegistry, Secret};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

/// The client token every fixture server accepts.
pub static TOKEN: LazyLock<Secret> = LazyLock::new(|| Secret::generate(CredentialKind::Client));

/// A registry holding [`TOKEN`] as the client token.
pub fn registry() -> CredentialRegistry {
    let registry = CredentialRegistry::new();
    registry.set_client_token(&TOKEN);
    registry
}

/// A client that sends `credential` as a bearer token on every request.
pub fn client_with(credential: &Secret) -> reqwest::Client {
    let mut headers = HeaderMap::new();
    let value = format!("Bearer {}", credential.expose());
    headers.insert(AUTHORIZATION,
                   HeaderValue::from_str(&value).expect("a credential is a valid header value"));
    reqwest::Client::builder().default_headers(headers)
                              .build()
                              .expect("the test client builds")
}

/// A client that presents [`TOKEN`].
pub fn client() -> reqwest::Client {
    client_with(&TOKEN)
}

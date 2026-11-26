//! Who a request comes from: the credentials Knot issues, the registry
//! that recognises them, and the record of requests it refused.
//!
//! Contract: `openspec/specs/mcp-server/spec.md` ("Client authentication",
//! "Credential kinds", "Credentials stay secret") and
//! `openspec/changes/mcp-auth/design.md` D1, D2, D8 and D10.
//!
//! This module decides *who*; what each principal may reach is
//! [`principal::Route`]'s business, and the HTTP checks that apply both are
//! in `server::guard`.

mod credential;
mod principal;
mod refusals;
mod registry;

pub use credential::{CredentialKind, Secret};
pub use principal::{Principal, Route};
pub use refusals::{RefusalLog, RefusalSource};
pub use registry::{AgentCredentials, CredentialRegistry};

#[cfg(test)]
mod tests;

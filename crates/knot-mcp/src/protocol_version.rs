//! Which MCP revision a connection speaks: the version `initialize` answers
//! with, and whether a later request's `MCP-Protocol-Version` header names
//! one the server will serve.
//!
//! The lists themselves are decisions and live in `consts`.

use crate::consts::{LEGACY_PROTOCOL_VERSIONS, PROTOCOL_VERSIONS};

/// The version to answer `initialize` with, given the one the client asked
/// for: the same version when the server implements it, otherwise the newest
/// it does. A client that cannot use the answer is expected to disconnect
/// (MCP 2025-11-25, Lifecycle, "Version Negotiation").
pub(crate) fn negotiate(requested: Option<&str>) -> &'static str {
    requested.and_then(|requested| {
                 PROTOCOL_VERSIONS.iter()
                                  .copied()
                                  .find(|supported| *supported == requested)
             })
             .unwrap_or(PROTOCOL_VERSIONS[0])
}

/// Whether a request's `MCP-Protocol-Version` header may be served. A missing
/// header is: the spec has the server assume 2025-03-26 then. A header naming
/// a version the server neither implements nor has negotiated in the past is
/// refused with 400 (MCP 2025-11-25, Transports, "Protocol Version Header").
pub(crate) fn header_is_acceptable(header: Option<&str>) -> bool {
    header.is_none_or(|version| {
              PROTOCOL_VERSIONS.contains(&version) || LEGACY_PROTOCOL_VERSIONS.contains(&version)
          })
}

#[cfg(test)]
mod tests {
    use super::{header_is_acceptable, negotiate};
    use crate::consts::PROTOCOL_VERSIONS;

    #[test]
    fn a_supported_version_is_echoed() {
        for version in PROTOCOL_VERSIONS {
            assert_eq!(negotiate(Some(version)), *version);
        }
    }

    #[test]
    fn an_unsupported_or_missing_version_gets_the_newest() {
        assert_eq!(negotiate(Some("2024-11-05")), "2025-11-25");
        assert_eq!(negotiate(Some("not-a-version")), "2025-11-25");
        assert_eq!(negotiate(None), "2025-11-25");
    }

    #[test]
    fn the_newest_supported_version_is_listed_first() {
        let mut sorted = PROTOCOL_VERSIONS.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(PROTOCOL_VERSIONS, sorted.as_slice());
    }

    #[test]
    fn a_header_is_refused_only_when_it_names_an_unknown_version() {
        assert!(header_is_acceptable(None));
        assert!(header_is_acceptable(Some("2025-06-18")));
        assert!(header_is_acceptable(Some("2024-11-05")),
                "Knot's own answer before #442");
        assert!(!header_is_acceptable(Some("2099-01-01")));
        assert!(!header_is_acceptable(Some("")));
    }
}

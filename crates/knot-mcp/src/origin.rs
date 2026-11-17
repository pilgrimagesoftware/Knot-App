//! Which `Origin` headers the server answers.
//!
//! Streamable HTTP requires a server to validate `Origin` on every incoming
//! connection and to answer an invalid one with 403 (MCP 2025-11-25,
//! Transports, "Security Warning"). The agents Knot launches (Claude Code,
//! Codex, agy, opencode) send no `Origin` at all, and neither do the hook
//! scripts that `curl` the status endpoints, so a request without one is
//! served. A request with one is served only when it comes from a page on
//! this machine's loopback interface.

use axum::http::Uri;

use crate::consts::LOOPBACK_ORIGIN_HOSTS;

/// Whether a request carrying `origin` (its `Origin` header, or `None` when
/// it sent none) may be served.
///
/// The opaque origin `null` (a sandboxed frame, a `file:` page) names no host
/// and is refused, as is anything that does not parse as an `http` or
/// `https` origin.
pub(crate) fn is_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin
    else {
        return true;
    };
    let Ok(uri) = origin.parse::<Uri>()
    else {
        return false;
    };
    let web_scheme = uri.scheme_str()
                        .is_some_and(|scheme| scheme == "http" || scheme == "https");
    web_scheme
    && uri.host().is_some_and(|host| {
                     LOOPBACK_ORIGIN_HOSTS.iter()
                                          .any(|allowed| host.eq_ignore_ascii_case(allowed))
                 })
}

#[cfg(test)]
mod tests {
    use super::is_allowed;

    #[test]
    fn a_request_without_an_origin_is_served() {
        assert!(is_allowed(None));
    }

    #[test]
    fn a_loopback_origin_is_served_on_any_port() {
        for origin in ["http://localhost",
                       "http://localhost:3000",
                       "https://LOCALHOST:8443",
                       "http://127.0.0.1:8767",
                       "http://[::1]:5173"]
        {
            assert!(is_allowed(Some(origin)), "{origin}");
        }
    }

    #[test]
    fn any_other_origin_is_refused() {
        for origin in ["https://example.com",
                       "http://localhost.example.com",
                       "http://127.0.0.1.example.com",
                       "http://192.168.1.10:8767",
                       "null",
                       "file://",
                       "chrome-extension://abcdef",
                       "",
                       "not an origin"]
        {
            assert!(!is_allowed(Some(origin)), "{origin}");
        }
    }
}

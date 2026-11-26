//! The checks a request passes before any handler sees it, in the order
//! `openspec/changes/mcp-auth/design.md` D4 fixes: the `Host` header, then
//! `Origin` (`routes::refuse_foreign_origin`), then the credential and the
//! URL's agent hint.
//!
//! What a refused request gets is the same everywhere: a status, a JSON-RPC
//! error with no id, and no change to any state. The principal a request
//! authenticated as rides on to the handlers as a request extension.

use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use super::AppState;
use super::routes::agent_from_query;
use crate::auth::{Principal, RefusalSource, Route};
use crate::consts;
use crate::log::Subject;
use crate::rpc::{JsonRpcId, JsonRpcResponse};

/// JSON-RPC 2.0 "Invalid Request", as every other refusal answers.
const INVALID_REQUEST: i64 = -32600;

/// Whether `host`, a request's `Host` header, names this server: a loopback
/// name with the port it listens on (`mcp-server` "Host validation").
pub(super) fn host_is_allowed(host: Option<&str>, port: u16) -> bool {
    let Some((name, named_port)) = host.and_then(|host| host.rsplit_once(':'))
    else {
        return false;
    };
    named_port.parse::<u16>() == Ok(port)
    && consts::LOOPBACK_HOSTS.iter()
                             .any(|allowed| name.eq_ignore_ascii_case(allowed))
}

/// Refuses a request whose `Host` does not name this server, before
/// anything else looks at it. Every route, `/health` included: a page that
/// rebinds a name of its own to 127.0.0.1 sends that name here.
pub(super) async fn refuse_foreign_host(State(state): State<AppState>, request: Request,
                                        next: Next)
                                        -> Response {
    let host = request.headers()
                      .get(header::HOST)
                      .and_then(|value| value.to_str().ok());
    if host_is_allowed(host, state.port) {
        return next.run(request).await;
    }
    refusal(StatusCode::FORBIDDEN, "Forbidden: Host not allowed")
}

/// The route a request is for, as far as credentials go; `None` for
/// `/health`, which needs none. A path the router does not serve still needs
/// a credential of some kind, so an unauthenticated caller cannot map the
/// server by its 404s.
fn route_of(method: &Method, path: &str) -> Option<Option<Route>> {
    match (method, path) {
        (_, "/health") => None,
        (_, "/") => Some(Some(Route::Info)),
        (_, "/mcp") => Some(Some(Route::Mcp)),
        (&Method::GET, consts::STATUS_PATH) => Some(Some(Route::StatusRead)),
        (&Method::POST, consts::STATUS_PATH | consts::REGISTER_PATH) => Some(Some(Route::Hook)),
        _ => Some(None),
    }
}

/// The value of an `Authorization: Bearer <value>` header, if it has one.
/// The scheme is case-insensitive (RFC 7235, section 2.1).
fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, credential) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer")
          .then(|| credential.trim())
          .filter(|credential| !credential.is_empty())
}

/// Authenticates a request (D4 steps 3 and 4) and hands its principal on.
///
/// - no credential, or one Knot did not issue or has revoked: 401 with the
///   `Bearer` challenge;
/// - a credential whose kind does not cover the route: 403;
/// - an agent's credential with a URL hint naming another agent: 403.
///
/// Each refusal is logged with the principal (or `none`) and counted
/// against the hint or the `User-Agent` (D10). The credential itself is
/// never logged.
pub(super) async fn authenticate(State(state): State<AppState>, mut request: Request, next: Next)
                                 -> Response {
    let Some(route) = route_of(request.method(), request.uri().path())
    else {
        return next.run(request).await;
    };
    let hint = agent_from_query(request.uri().query());
    let principal =
        bearer(request.headers()).and_then(|value| state.credentials.authenticate(value));
    let refused = match principal {
        None => Some((StatusCode::UNAUTHORIZED, "Unauthorized: a Knot credential is required")),
        Some(principal) if route.is_some_and(|route| !principal.may_reach(route)) => {
            Some((StatusCode::FORBIDDEN, "Forbidden: this credential does not cover the route"))
        }
        Some(principal) if hint_disagrees(principal, hint) => {
            Some((StatusCode::FORBIDDEN, "Forbidden: the credential names another agent"))
        }
        Some(_) => None,
    };
    if let Some((status, message)) = refused {
        record_refusal(&state, &request, principal, hint, status);
        return refusal(status, message);
    }
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    next.run(request).await
}

/// Whether `principal` speaks for one agent and the URL names another.
fn hint_disagrees(principal: Principal, hint: Option<Uuid>) -> bool {
    matches!((principal.agent(), hint), (Some(agent), Some(hint)) if agent != hint)
}

fn record_refusal(state: &AppState, request: &Request, principal: Option<Principal>,
                  hint: Option<Uuid>, status: StatusCode) {
    let user_agent = request.headers()
                            .get(header::USER_AGENT)
                            .and_then(|value| value.to_str().ok());
    // Counted against the hinted agent only when Knot launched it: the hint
    // comes from an unauthenticated URL, and anything else would let a
    // caller grow the table with made-up ids.
    let hint = hint.filter(|agent| state.credentials.has_agent_pair(*agent));
    state.refusals.record(RefusalSource::of(hint, user_agent));
    let who = principal.map_or_else(|| "none".to_owned(), |principal| principal.to_string());
    let line = format!("[{who}] {} {} refused {}",
                       request.method(),
                       request.uri().path(),
                       status.as_u16());
    eprintln!("knot-mcp: {line}");
    if let Some(log) = state.log.as_ref() {
        log.warn(Subject::Auth, line);
    }
}

/// A refusal: `status`, a JSON-RPC error with no id, and on a 401 the
/// `Bearer` challenge (RFC 6750, section 3).
fn refusal(status: StatusCode, message: &str) -> Response {
    let body = JsonRpcResponse::error(None::<JsonRpcId>, INVALID_REQUEST, message);
    let mut response = (status, axum::Json(body)).into_response();
    if status == StatusCode::UNAUTHORIZED {
        response.headers_mut().insert(header::WWW_AUTHENTICATE,
                                      HeaderValue::from_static(consts::AUTH_CHALLENGE));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::host_is_allowed;

    #[test]
    fn only_a_loopback_name_on_the_listening_port_is_served() {
        assert!(host_is_allowed(Some("127.0.0.1:8767"), 8767));
        assert!(host_is_allowed(Some("localhost:8767"), 8767));
        assert!(host_is_allowed(Some("LOCALHOST:8767"), 8767));

        for host in [None,
                     Some("other.example:8767"),
                     Some("127.0.0.1:8768"),
                     Some("127.0.0.1"),
                     Some("localhost"),
                     Some("127.0.0.1.example:8767"),
                     Some("[::1]:8767"),
                     Some("")]
        {
            assert!(!host_is_allowed(host, 8767), "{host:?}");
        }
    }
}

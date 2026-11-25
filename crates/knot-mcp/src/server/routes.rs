//! The HTTP surface of [`super::McpServer`]: the router, the `Origin` guard,
//! the health, info, status and hook endpoints, and JSON-RPC and SSE on
//! `/mcp`. It does not own the listener or the server's lifecycle; those stay
//! in [`super`].

use std::sync::atomic::Ordering;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Extension, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use uuid::Uuid;

use super::{AppState, guard};
use crate::auth::Principal;
use crate::consts;
use crate::hooks::{AgentHookHandler, HookRequest};
use crate::log::Subject;
use crate::origin;
use crate::protocol_version;
use crate::rpc::{self, JsonRpcId, JsonRpcRequest, JsonRpcResponse};
use crate::status::{self, AgentStatusEntry};
use crate::tools::Caller;

const SESSION_HEADER: &str = "Mcp-Session-Id";
/// JSON-RPC 2.0 "Invalid Request": the message is well formed but cannot be
/// served.
const INVALID_REQUEST: i64 = -32600;

/// The router, with its checks in the order D4 fixes. A layer added later
/// runs earlier, so the list below reads bottom-up: `Host`, then `Origin`,
/// then the credential.
pub(super) fn build_router(state: AppState) -> Router {
    Router::new().route("/health", get(health))
                 .route("/", get(info))
                 .route("/mcp", post(mcp_rpc).get(mcp_sse))
                 .route(consts::STATUS_PATH,
                        get(agent_status_endpoint).post(agent_status_hook_endpoint))
                 .route(consts::REGISTER_PATH, post(agent_register_endpoint))
                 .fallback(not_found)
                 .layer(axum::middleware::from_fn_with_state(state.clone(), guard::authenticate))
                 .layer(axum::middleware::from_fn(refuse_foreign_origin))
                 .layer(axum::middleware::from_fn_with_state(state.clone(),
                                                             guard::refuse_foreign_host))
                 .with_state(state)
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

/// Answers a request whose `Origin` names anything but this machine's
/// loopback interface with 403 and a JSON-RPC error with no id, and passes
/// every other request on (MCP 2025-11-25, Transports, "Security Warning").
/// Every route is covered: the hook endpoints change agent state as `/mcp`
/// does.
async fn refuse_foreign_origin(request: axum::extract::Request, next: axum::middleware::Next)
                               -> Response {
    let origin = request.headers()
                        .get(header::ORIGIN)
                        .map(|value| value.to_str().unwrap_or_default());
    if origin::is_allowed(origin) {
        return next.run(request).await;
    }
    let response = JsonRpcResponse::error(None::<JsonRpcId>,
                                          INVALID_REQUEST,
                                          "Forbidden: Origin not allowed");
    (StatusCode::FORBIDDEN, axum::Json(response)).into_response()
}

async fn health() -> &'static str {
    "OK"
}

async fn info() -> Response {
    axum::Json(serde_json::json!({
                   "name": consts::SERVER_NAME,
                   "version": consts::SERVER_VERSION,
               })).into_response()
}

async fn agent_status_endpoint(State(state): State<AppState>) -> axum::Json<Vec<AgentStatusEntry>> {
    let agents = (state.agents)();
    axum::Json(status::agent_status(&agents))
}

async fn agent_register_endpoint(State(state): State<AppState>,
                                 Extension(principal): Extension<Principal>, body: Bytes)
                                 -> Response {
    hook_response(&state, principal, &body, HookAction::Register)
}

async fn agent_status_hook_endpoint(State(state): State<AppState>,
                                    Extension(principal): Extension<Principal>, body: Bytes)
                                    -> Response {
    hook_response(&state, principal, &body, HookAction::Status)
}

enum HookAction {
    Register,
    Status,
}

fn hook_response(state: &AppState, principal: Principal, body: &[u8], action: HookAction)
                 -> Response {
    let request: HookRequest = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    let agent = match request.agent_id()
                             .and_then(|agent| request.validate_agent().map(|()| agent))
    {
        Ok(agent) => agent,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    if !may_post_hooks_for(state, principal, agent) {
        return (StatusCode::FORBIDDEN, "Forbidden: this credential cannot report for that agent")
            .into_response();
    }

    let handler: Option<&dyn AgentHookHandler> = state.hooks.as_deref();
    let Some(handler) = handler
    else {
        return (StatusCode::SERVICE_UNAVAILABLE, "Agent hooks are not configured").into_response();
    };
    let result = match action {
        HookAction::Register => handler.register(&request),
        HookAction::Status => handler.status(&request),
    };
    match result {
        Ok(value) => axum::Json(value).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

/// Whether `principal` may post a hook event for `agent` (D6): an agent's
/// hook credential only for that agent, and the client token only for an
/// agent Knot did not launch - one with no issued pair and no session opened
/// with its own credential.
fn may_post_hooks_for(state: &AppState, principal: Principal, agent: Uuid) -> bool {
    match principal {
        Principal::AgentHooks(own) => own == agent,
        // Not for an agent Knot launched, even before its adapter connects
        // or after its connection lapses: holding a pair is what counts.
        Principal::External => {
            !state.credentials.has_agent_pair(agent) && !state.sessions.held_by_credential(agent)
        }
        Principal::Agent(_) => false,
    }
}

fn sse_response(event: &str, data: &str, session_id: Option<&str>) -> Response {
    let body = format!("event: {event}\ndata: {data}\n\n");
    let mut response = Response::builder().status(StatusCode::OK)
                                          .header(header::CONTENT_TYPE, "text/event-stream")
                                          .header(header::CACHE_CONTROL, "no-cache")
                                          .header(header::CONNECTION, "keep-alive");
    if let Some(id) = session_id
       && let Ok(value) = HeaderValue::from_str(id)
    {
        response = response.header(SESSION_HEADER, value);
    }
    response.body(axum::body::Body::from(body)).unwrap()
}

async fn mcp_sse(headers: HeaderMap) -> Response {
    if let Some(refused) = refuse_unknown_protocol_version(&headers) {
        return refused;
    }
    sse_response("connected", "{\"status\":\"connected\"}", None)
}

/// A 400 for a request whose `MCP-Protocol-Version` header names a version
/// the server does not serve, or `None` to carry on. A header that is not
/// valid text counts as an invalid version.
///
/// Only a request that carries an `Mcp-Session-Id` is held to it: the header
/// names the version negotiated at `initialize`, and a request with no
/// session has negotiated nothing. Claude Code and agy both send
/// `server/discover` with `MCP-Protocol-Version: 2026-07-28` before they
/// initialize (#431, #442), and refusing that probe would turn their
/// method-not-found fallback into a failed connection.
fn refuse_unknown_protocol_version(headers: &HeaderMap) -> Option<Response> {
    if !headers.contains_key(SESSION_HEADER) {
        return None;
    }
    let version = match headers.get(consts::PROTOCOL_VERSION_HEADER)
                               .map(HeaderValue::to_str)
    {
        None => None,
        Some(Ok(version)) => Some(version),
        Some(Err(_)) => Some(""),
    };
    if protocol_version::header_is_acceptable(version) {
        return None;
    }
    let message = format!("Unsupported {}: {}",
                          consts::PROTOCOL_VERSION_HEADER,
                          version.unwrap_or_default());
    Some(json_rpc_error_response(INVALID_REQUEST, message))
}

/// The agent a request's URL names, from `?agent=<id>`; `None` for a URL
/// that names none or names it badly.
pub(crate) fn agent_from_query(query: Option<&str>) -> Option<Uuid> {
    query?.split('&')
          .filter_map(|pair| pair.split_once('='))
          .find(|(key, _)| *key == knot_core::consts::MCP_AGENT_QUERY)
          .and_then(|(_, value)| Uuid::parse_str(value).ok())
}

/// The connection a request arrived on, as a [`Caller`] - its session and
/// the agent that session is bound to - and the session id to answer with,
/// if any.
///
/// Who the connection is comes from `principal`, never from the URL (D5).
/// An agent's tool credential opens a session bound to that agent, which
/// replaces one it held before: a restarted agent's new connection takes
/// over from its old one. The client token opens an unbound session. A
/// session id the client sends is honoured only when the same principal
/// opened it; any other id is treated as unknown, so presenting someone
/// else's session id gets a fresh session of one's own. A session is only
/// reported to the tools as kept when the client will send it again - one
/// from `initialize`, or one it sent and the server still knows.
///
/// A request that is not `initialize` and carries no session id is answered
/// with none (#431). Streamable HTTP assigns a session at initialization
/// (MCP 2025-11-25, Transports, "Session Management"); a client probing
/// before initializing - agy's Go client sends `server/discover` first - and
/// handed an id there sends it on `initialize`, gets a different one back,
/// and abandons the connection without ever listing tools.
fn connection(state: &AppState, is_initialize: bool, sent: Option<String>, principal: Principal)
              -> (Caller, Option<String>) {
    let sessions = &state.sessions;
    let opened = || sessions.create_session(principal).id;
    let (session_id, kept) = match sent {
        _ if is_initialize => (Some(opened()), true),
        Some(id) if sessions.session_for(&id, principal).is_some() => {
            sessions.touch(&id);
            (Some(id), true)
        }
        // The client's session outlived the server's TTL (e.g. a paused
        // session resuming after `DEFAULT_SESSION_TIMEOUT`), or was never
        // this principal's. Reissue a fresh session instead of erroring so
        // the client self-heals without a manual `/mcp reconnect` or
        // restart.
        Some(_) => (Some(opened()), false),
        None => (None, false),
    };
    let bound = session_id.as_deref()
                          .and_then(|id| sessions.session(id))
                          .map(|session| session.agent_id)
                          .filter(|agent| !agent.is_nil());
    let own = match principal {
        Principal::Agent(agent) => Some(agent),
        Principal::AgentHooks(_) | Principal::External => None,
    };
    let caller = Caller { agent:       own.or(bound),
                          session_id:  session_id.clone().filter(|_| kept),
                          sessions:    sessions.clone(),
                          credentials: state.credentials.clone(), };
    (caller, session_id)
}

async fn mcp_rpc(State(state): State<AppState>, Extension(principal): Extension<Principal>,
                 headers: HeaderMap, body: Bytes)
                 -> Response {
    // Counted before parsing: a request that arrives malformed still
    // arrived, and a heartbeat reporting zero while a client hammers the
    // server with nonsense would be the more misleading of the two.
    state.served.fetch_add(1, Ordering::Relaxed);
    state.sessions.cleanup_stale_default();
    let session_id = headers.get(SESSION_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_string);
    let accepts_sse = headers.get(header::ACCEPT)
                             .and_then(|v| v.to_str().ok())
                             .is_some_and(|v| v.contains("text/event-stream"));

    let request: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return json_rpc_error_response(-32700, format!("Parse error: {e}")),
    };
    // `initialize` is where the version is chosen, so only the requests
    // after it are held to one. A client re-initializing on an old session
    // still names the version it is about to replace.
    if request.method != "initialize"
       && let Some(refused) = refuse_unknown_protocol_version(&headers)
    {
        return refused;
    }

    let (caller, response_session_id) = connection(&state,
                                                   request.method == "initialize",
                                                   session_id,
                                                   principal);
    // An agent's own credential is proof of who this is, so it registers the
    // agent with no turn of its own. Repeated on every request rather than
    // once, so an agent the registry did not hold yet is registered as soon
    // as it does. The client token proves no agent and registers none.
    if let Principal::Agent(agent) = principal {
        state.catalog.connected(agent);
    }

    if request.method.starts_with("notifications/") {
        return StatusCode::ACCEPTED.into_response();
    }

    // Named by the caller's agent, not by `response_session_id`: the id is
    // the credential a client sends back as `Mcp-Session-Id`, and the logs
    // end up in public bug reports (see `log::redact`).
    let who = principal;
    eprintln!("knot-mcp: [{who}] -> {}", request.method);
    if let Some(log) = state.log.as_ref() {
        log.info(Subject::Request, format!("[{who}] {}", request.method));
    }
    let response = rpc::dispatch(&request,
                                 state.catalog.as_ref(),
                                 state.log.as_ref(),
                                 &caller).await;
    let outcome = if response.error.is_some() {
        "error"
    }
    else {
        "ok"
    };
    eprintln!("knot-mcp: [{who}] <- {} {outcome}", request.method);
    if let Some(log) = state.log.as_ref() {
        log.info(Subject::Response,
                 format!("[{who}] {} {outcome}", request.method));
    }

    if accepts_sse {
        let data = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        return sse_response("message", &data, response_session_id.as_deref());
    }

    let mut resp = axum::Json(response).into_response();
    if let Some(id) = response_session_id.as_deref()
       && let Ok(value) = HeaderValue::from_str(id)
    {
        resp.headers_mut().insert(SESSION_HEADER, value);
    }
    resp
}

fn json_rpc_error_response(code: i64, message: String) -> Response {
    let response = JsonRpcResponse::error(None::<JsonRpcId>, code, message);
    (StatusCode::BAD_REQUEST, axum::Json(response)).into_response()
}

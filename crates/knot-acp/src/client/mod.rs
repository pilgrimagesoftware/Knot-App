//! ACP semantics (`initialize`, session lifecycle, updates, permissions)
//! built on top of the generic [`Transport`].

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::error::{AcpError, Result};
use crate::protocol::{
    AgentCapabilities, AgentInfo, ClientCapabilities, ConfigOption, FormAnswer, InitializeParams,
    InitializeResult, MCP_SERVER_NAME, PROTOCOL_VERSION, PermissionDecision, PermissionRequest,
    QuestionRequest, SessionUpdate, StopReason,
};
use crate::transport::{Transport, TransportEvent};

mod events;
mod inbound;
mod mcp_endpoint;
mod pending;
mod steer;
pub use events::{NewSession, SessionEvent};
use inbound::Inbound;
pub use mcp_endpoint::McpEndpoint;
use pending::PendingInputs;

/// Cheap to clone - `transport` and `pending` are both `Arc`,
/// so every clone dispatches through the same underlying connection. Lets
/// a caller holding one behind a `Mutex` extract an owned handle to
/// `.await` on without keeping the lock held across the await point.
#[derive(Clone)]
pub struct AcpClient {
    transport:           Arc<Transport>,
    capabilities:        AgentCapabilities,
    /// The agent's name and version from `initialize`, if it sent them.
    /// Behind an `Arc` because the client is cloned for every request, and
    /// three owned strings would otherwise be copied each time.
    agent_info:          Option<Arc<AgentInfo>>,
    /// Session Config Options declared on `initialize`, if any - seeds a
    /// new session's config options before `session/new`'s own (possibly
    /// richer, per-session) list arrives.
    init_config_options: Vec<ConfigOption>,
    /// Permission requests and forms the agent is waiting on, and the
    /// sessions this client has open.
    pending:             Arc<PendingInputs>,
    /// The same channel the transport loop feeds. `session_prompt` needs
    /// it because ACP ends a turn by *responding* to `session/prompt`
    /// with a stop reason rather than sending a `session/update`, so the
    /// turn-end event has to be synthesized from that response.
    events:              mpsc::UnboundedSender<SessionEvent>,
    /// The `_meta` sent with `session/new` and `session/load`: how an
    /// adapter takes per-session options it has no ACP field for - the
    /// user's CLI flags, for `claude-agent-acp`. `None` sends none.
    session_meta:        Option<Value>,
}

impl AcpClient {
    /// Spawns `command`, negotiates the protocol version, and returns the
    /// client plus the ordered session-event stream. Fails closed (per
    /// `acp-client`'s capability-negotiation requirement) if the agent
    /// reports an unsupported protocol version.
    pub async fn connect(command: Command)
                         -> Result<(Self, mpsc::UnboundedReceiver<SessionEvent>)> {
        let (transport, mut transport_events) = Transport::spawn(command)?;

        let init_params = InitializeParams { protocol_version:    PROTOCOL_VERSION,
                                             client_capabilities: ClientCapabilities::default(), };
        let raw = transport.request("initialize",
                                    Some(serde_json::to_value(init_params).expect("serializable")))
                           .await?;
        let init_result: InitializeResult = serde_json::from_value(raw).map_err(|error| {
                                                                           AcpError::Rpc {
                code: -32600,
                message: format!("malformed initialize response: {error}"),
            }
                                                                       })?;
        if init_result.protocol_version != PROTOCOL_VERSION {
            return Err(AcpError::UnsupportedProtocolVersion { client: PROTOCOL_VERSION,
                                                              agent:  init_result.protocol_version, });
        }

        let pending = Arc::new(PendingInputs::default());
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let events_for_client = events_tx.clone();

        // `Weak`, not `Arc`: this pump only ends when the transport's
        // `events_tx` drops, and that cannot happen while the pump itself
        // holds the transport alive. Holding a strong reference here was
        // the second half of the cycle that orphaned adapter subprocesses
        // (see `Transport::tasks`).
        let transport_for_loop = Arc::downgrade(&transport);
        let pending_for_loop = Arc::clone(&pending);
        tokio::spawn(async move {
            while let Some(event) = transport_events.recv().await {
                let Some(transport_for_loop) = transport_for_loop.upgrade()
                else {
                    break;
                };
                match event {
                    TransportEvent::Notification { method, params }
                        if method == "session/update" =>
                    {
                        let update = SessionUpdate::from_params(params.unwrap_or(Value::Null));
                        let _ = events_tx.send(SessionEvent::Update(update));
                    }
                    TransportEvent::Notification { method, params }
                        if method == "$/cancel_request" =>
                    {
                        Inbound { transport: transport_for_loop,
                                  events:    events_tx.clone(),
                                  pending:   Arc::clone(&pending_for_loop), }.cancel_request(params.as_ref());
                    }
                    TransportEvent::Notification { .. } => {}
                    TransportEvent::Request { id, method, params } => {
                        Inbound { transport: transport_for_loop,
                                  events:    events_tx.clone(),
                                  pending:   Arc::clone(&pending_for_loop), }.request(id, &method,
                                                                                      params)
                                                                             .await;
                    }
                    TransportEvent::Ended(cause) => {
                        // Per the "session closed while a permission request
                        // is pending" scenario: any still-pending permission
                        // resolves to Deny, and any form to `cancel`, rather
                        // than hanging.
                        pending_for_loop.end_all();
                        let _ = events_tx.send(SessionEvent::Ended(cause));
                        break;
                    }
                }
            }
        });

        let mut capabilities = init_result.capabilities;
        capabilities.supports_steering =
            crate::protocol::steering::advertises_steering(init_result.meta.as_ref());
        Ok((Self { transport,
                   capabilities,
                   agent_info: init_result.agent_info.map(Arc::new),
                   init_config_options: init_result.config_options,
                   pending,
                   events: events_for_client,
                   session_meta: None },
            events_rx))
    }

    pub fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    /// The agent's name and version as it reported them on `initialize`.
    /// `None` when it did not - the spec only says it SHOULD.
    pub fn agent_info(&self) -> Option<&AgentInfo> {
        self.agent_info.as_deref()
    }

    /// The adapter subprocess's process id, while the connection is live.
    ///
    /// A panel agent's session root: what the processes section enumerates
    /// descendants of. `None` once the adapter has been reaped.
    pub fn process_id(&self) -> Option<u32> {
        self.transport.process_id()
    }

    /// Opens every later session with `meta` as its `_meta`.
    #[must_use]
    pub fn with_session_meta(mut self, meta: Option<Value>) -> Self {
        self.session_meta = meta;
        self
    }

    pub async fn session_new(&self, cwd: &str, mcp: Option<McpEndpoint<'_>>) -> Result<NewSession> {
        let params = self.session_params(json!({ "cwd": cwd,
                                                 "mcpServers": self.mcp_servers(mcp) }));
        let raw = self.transport.request("session/new", Some(params)).await?;
        let session_id = session_id_from(&raw)?;
        self.pending.open_session(&session_id);
        let config_options = config_options_from(&raw, &self.init_config_options);
        Ok(NewSession { session_id,
                        config_options })
    }

    /// Resumes a prior session. Returns a typed "not supported" error
    /// without sending the request when the agent's capabilities don't
    /// advertise `session/load` support.
    pub async fn session_load(&self, session_id: &str, cwd: &str, mcp: Option<McpEndpoint<'_>>)
                              -> Result<NewSession> {
        if !self.capabilities.supports_resume {
            return Err(AcpError::ResumeNotSupported);
        }
        let params = self.session_params(json!({ "sessionId": session_id, "cwd": cwd,
                                                 "mcpServers": self.mcp_servers(mcp) }));
        let raw = self.transport.request("session/load", Some(params)).await?;
        // ACP's `LoadSessionResponse` carries no `sessionId`: a loaded
        // session keeps the id it was asked for. `codex-acp` 2.0.0 answers
        // exactly that way, and treating the absence as an error made every
        // Codex load fall back to a fresh session. An adapter that does name
        // one (Claude's does) is taken at its word.
        let session_id = raw.get("sessionId")
                            .and_then(Value::as_str)
                            .map_or_else(|| session_id.to_owned(), str::to_owned);
        self.pending.open_session(&session_id);
        let config_options = config_options_from(&raw, &self.init_config_options);
        Ok(NewSession { session_id,
                        config_options })
    }

    /// `params` with the session's `_meta`, when it has one.
    fn session_params(&self, mut params: Value) -> Value {
        if let (Some(meta), Some(object)) = (&self.session_meta, params.as_object_mut()) {
            object.insert("_meta".to_owned(), meta.clone());
        }
        params
    }

    /// The `mcpServers` array for a `session/new`/`session/load` request:
    /// one entry naming Knot's own HTTP MCP server, in the ACP spec's
    /// `McpServerHttp` shape (`type`/`name`/`url`/`headers`, `headers`
    /// required even when empty - https://agentclientprotocol.com/protocol/v1/schema),
    /// when `mcp_url` is given AND the agent declared `mcpCapabilities.http`
    /// on `initialize` (sending an `http`-type entry to an agent that
    /// hasn't declared support for it is a protocol violation - confirmed
    /// live: opencode/gemini reject it with `Invalid params`/`Internal
    /// error`). Empty otherwise. `mcpServers` itself is still required by
    /// at least the Gemini CLI adapter even when empty. The agent's tool
    /// credential, when it has one, goes in `headers` (see [`McpEndpoint`]).
    fn mcp_servers(&self, mcp: Option<McpEndpoint<'_>>) -> Value {
        match mcp {
            Some(endpoint) if self.capabilities.mcp_capabilities.http => {
                json!([{ "type": "http", "name": MCP_SERVER_NAME, "url": endpoint.url,
                         "headers": endpoint.headers() }])
            }
            _ => json!([]),
        }
    }

    /// Applies one Session Config Option selection (mode, model, effort,
    /// ...) and returns the agent's updated full list, per the stabilized
    /// Session Config Options `session/set_config_option` response shape.
    pub async fn session_set_config_option(&self, session_id: &str, config_id: &str, value: &str)
                                           -> Result<Vec<ConfigOption>> {
        let raw = self.transport
                      .request("session/set_config_option",
                               Some(json!({ "sessionId": session_id, "configId": config_id,
                                          "type": "id", "value": value })))
                      .await?;
        Ok(raw.get("configOptions")
              .cloned()
              .map(serde_json::from_value)
              .and_then(std::result::Result::ok)
              .unwrap_or_default())
    }

    /// Sends one prompt and waits for the turn to finish.
    ///
    /// ACP ends a prompt turn by responding to *this request* with a
    /// `stopReason` - there is no `turn_end` session update
    /// (<https://agentclientprotocol.com/protocol/prompt-turn>, "Check for
    /// Completion"). So the turn-end event callers rely on is synthesized
    /// from the response here. It is emitted even when the request fails,
    /// because a caller that gates input on "a turn is in flight" would
    /// otherwise stay blocked forever on a failed prompt.
    pub async fn session_prompt(&self, session_id: &str, text: &str) -> Result<()> {
        let result = self.transport
                         .request("session/prompt", Some(json!({ "sessionId": session_id, "prompt": [{ "type": "text", "text": text }] })))
                         .await;
        let stop_reason = match &result {
            Ok(raw) => StopReason::parse(raw.get("stopReason").and_then(Value::as_str)),
            Err(_) => StopReason::Error,
        };
        // Beside the transport's `<- session/prompt (id N) ok`: whether the
        // agent finished or stopped early is otherwise not in the log (#22).
        // No session id: CodeQL treats it as a secret.
        eprintln!("knot-acp: session/prompt ended: stopReason {stop_reason}");
        let _ = self.events
                    .send(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason }));
        result.map(|_| ())
    }

    /// Cancels the turn in flight.
    ///
    /// Sends `session/cancel`, then answers every permission request still
    /// pending with the `cancelled` outcome, and every form with `cancel`. The
    /// spec makes the second half the client's job: "The Client MUST
    /// respond to all pending `session/request_permission` requests with
    /// the `cancelled` outcome" (<https://agentclientprotocol.com/protocol/prompt-turn>,
    /// "Cancellation"). An agent blocked on one cannot finish aborting, so
    /// without it the turn's `session/prompt` never answered and Stop did
    /// nothing (#267).
    pub async fn session_cancel(&self, session_id: &str) -> Result<()> {
        let sent = self.transport
                       .notify("session/cancel", Some(json!({ "sessionId": session_id })))
                       .await;
        self.pending.cancel_all();
        sent
    }

    /// Answers a pending `session/request_permission` request surfaced via
    /// [`SessionEvent::PermissionRequest`].
    pub fn answer_permission(&self, request: &PermissionRequest, decision: PermissionDecision) {
        self.pending
            .answer_permission(&request.rpc_id.to_string(), decision);
    }

    /// Answers a pending form surfaced via [`SessionEvent::Question`]. Does
    /// nothing once it has been answered, cancelled or withdrawn.
    pub fn answer_question(&self, request: &QuestionRequest, answer: FormAnswer) {
        self.pending
            .answer_question(&request.rpc_id.to_string(), answer);
    }

    /// Closes the session by terminating the adapter subprocess. Any
    /// permission request still pending resolves to Deny, and any form to
    /// `cancel` (see the
    /// `Ended` handling in [`AcpClient::connect`]'s background loop)
    /// rather than being left unanswered.
    pub async fn close(&self) {
        self.transport.close().await;
    }
}

fn session_id_from(raw: &Value) -> Result<String> {
    raw.get("sessionId")
       .and_then(Value::as_str)
       .map(str::to_owned)
       .ok_or_else(|| AcpError::Rpc { code:    -32600,
                                      message: "session response missing sessionId".to_owned(), })
}

/// A `session/new`/`session/load` response's own `configOptions`, falling
/// back to whatever `initialize` already declared when the per-session
/// response omits them (the stabilized spec allows declaring them in
/// either place).
fn config_options_from(raw: &Value, init_config_options: &[ConfigOption]) -> Vec<ConfigOption> {
    raw.get("configOptions")
       .cloned()
       .map(serde_json::from_value)
       .and_then(std::result::Result::ok)
       .unwrap_or_else(|| init_config_options.to_vec())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod question_tests;

//! Requests and notifications the agent sends the client: permission
//! requests, form elicitations, `$/cancel_request`, and the decline for any
//! request this client does not handle.

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::SessionEvent;
use super::pending::{PendingInputs, PermissionAnswer, QuestionAnswer};
use crate::protocol::elicitation::parse_form;
use crate::protocol::{
    JsonRpcErrorPayload, PermissionDecision, PermissionOption, PermissionRequest,
};
use crate::transport::Transport;

/// JSON-RPC "method not found".
const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC "invalid params": a form this client cannot show.
const INVALID_PARAMS: i64 = -32602;
/// ACP's answer to a request its sender cancelled
/// (<https://agentclientprotocol.com/protocol/v1/cancellation>; the SDK's
/// `RequestError.requestCancelled`).
const REQUEST_CANCELLED: i64 = -32800;

/// What the transport pump needs to answer the agent and tell the caller.
pub(super) struct Inbound {
    pub(super) transport: Arc<Transport>,
    pub(super) events:    mpsc::UnboundedSender<SessionEvent>,
    pub(super) pending:   Arc<PendingInputs>,
}

impl Inbound {
    pub(super) async fn request(self, id: Value, method: &str, params: Option<Value>) {
        let params = params.unwrap_or(Value::Null);
        match method {
            "session/request_permission" => self.permission(id, &params),
            "elicitation/create" => self.question(id, &params).await,
            _ => {
                // Unrecognized server-to-client request: decline rather than
                // leaving it (and the agent) hanging.
                let _ = self.transport
                            .respond(id, Err(error(METHOD_NOT_FOUND, "method not supported")))
                            .await;
            }
        }
    }

    /// `$/cancel_request`: the agent no longer wants an answer to
    /// `params.requestId`. One still pending is answered `-32800` by the task
    /// waiting on it; an unknown id is ignored.
    pub(super) fn cancel_request(&self, params: Option<&Value>) {
        if let Some(id) = params.and_then(|params| params.get("requestId")) {
            self.pending.withdraw(&id.to_string());
        }
    }

    fn permission(self, id: Value, params: &Value) {
        // The spec nests the tool call: `params.toolCall.toolCallId`
        // (<https://agentclientprotocol.com/protocol/tool-calls>,
        // "Requesting Permission"). The flat spelling is kept as a fallback
        // for adapters that send it.
        let tool_call_id = params.get("toolCall")
                                 .and_then(|call| call.get("toolCallId"))
                                 .or_else(|| params.get("toolCallId"))
                                 .and_then(Value::as_str)
                                 .unwrap_or_default()
                                 .to_owned();
        let tool_call_title = params.get("toolCall")
                                    .and_then(|call| call.get("title"))
                                    .and_then(Value::as_str)
                                    .map(str::to_owned);
        let options: Vec<PermissionOption> =
            serde_json::from_value(params.get("options")
                                         .cloned()
                                         .unwrap_or(Value::Array(vec![]))).unwrap_or_default();
        let answer = self.pending.add_permission(id.to_string());
        let _ =
            self.events
                .send(SessionEvent::PermissionRequest(PermissionRequest { rpc_id: id.clone(),
                                                                          tool_call_id,
                                                                          tool_call_title,
                                                                          options:
                                                                              options.clone() }));
        tokio::spawn(async move {
            let answer = answer.await
                               .unwrap_or(PermissionAnswer::Decided(PermissionDecision::Deny));
            // Every way a request is answered ends here - the user's
            // decision, Stop's `cancelled`, the session ending, the agent
            // withdrawing it - so this is the one place that can say it is
            // no longer pending, whichever it was.
            let _ = self.events
                        .send(SessionEvent::InputResolved { rpc_id: id.clone() });
            let _ = self.transport
                        .respond(id, permission_result(answer, &options))
                        .await;
        });
    }

    async fn question(self, id: Value, params: &Value) {
        let request = match parse_form(id.clone(), params) {
            Ok(request) if self.pending.has_session(&request.session_id) => request,
            Ok(request) => {
                let message = format!("unknown session {:?}", request.session_id);
                let _ = self.transport
                            .respond(id, Err(error(INVALID_PARAMS, &message)))
                            .await;
                return;
            }
            Err(refusal) => {
                eprintln!("knot-acp: refused elicitation/create: {refusal}");
                let _ = self.transport
                            .respond(id, Err(error(INVALID_PARAMS, &refusal.to_string())))
                            .await;
                return;
            }
        };
        let answer = self.pending.add_question(id.to_string());
        let _ = self.events.send(SessionEvent::Question(request));
        tokio::spawn(async move {
            // A dropped sender only happens with the whole client gone;
            // `cancel` is what the session ending answers anyway.
            let answer =
                answer.await
                      .unwrap_or(QuestionAnswer::Answered(crate::protocol::FormAnswer::Cancelled));
            let _ = self.events
                        .send(SessionEvent::InputResolved { rpc_id: id.clone() });
            let result = match answer {
                QuestionAnswer::Answered(answer) => Ok(answer.to_result()),
                QuestionAnswer::Withdrawn => Err(error(REQUEST_CANCELLED, "request cancelled")),
            };
            let _ = self.transport.respond(id, result).await;
        });
    }
}

fn error(code: i64, message: &str) -> JsonRpcErrorPayload {
    JsonRpcErrorPayload { code,
                          message: message.to_owned(),
                          data: None }
}

/// The `session/request_permission` response for `answer`: the chosen
/// option, the `cancelled` outcome for a cancelled turn
/// (<https://agentclientprotocol.com/protocol/tool-calls>, "Requesting
/// Permission"), or `-32800` for one the agent withdrew. A decision with no
/// option on offer cannot reach here from the UI, which offers only what the
/// request does; one that does anyway - or a dropped channel - is answered as
/// a refusal, never left unanswered.
fn permission_result(answer: PermissionAnswer, options: &[PermissionOption])
                     -> std::result::Result<Value, JsonRpcErrorPayload> {
    let decision = match answer {
        PermissionAnswer::Decided(decision) => decision,
        PermissionAnswer::Cancelled => return Ok(json!({ "outcome": { "outcome": "cancelled" } })),
        PermissionAnswer::Withdrawn => return Err(error(REQUEST_CANCELLED, "request cancelled")),
    };
    let outcome = decision.option_id(options)
                          .or_else(|| PermissionDecision::Deny.option_id(options))
                          .unwrap_or_else(|| "deny".to_owned());
    Ok(json!({ "outcome": { "outcome": "selected", "optionId": outcome } }))
}

//! Form elicitation through a live (fake) agent: the capability on
//! `initialize`, the question event, each way a form is answered, and
//! `$/cancel_request`.
//!
//! The agent logs every line it receives, so assertions read what actually
//! went over the wire. `session/prompt` makes it ask (it never answers the
//! prompt), and `session/set_config_option` makes it withdraw request 200 -
//! a cue the test controls, rather than a sleep.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::process::Command;

use super::*;
use crate::protocol::{AnswerValue, FormAnswer};

/// The elicitation the agent sends as request 200, for session `session`.
fn question_line(session: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": 200, "method": "elicitation/create", "params": {
        "mode": "form", "sessionId": session, "toolCallId": "toolu_1",
        "message": "Please answer the following questions.",
        "requestedSchema": { "type": "object", "properties": {
            "question_0": { "type": "string", "title": "Color", "description": "Which color?",
                            "oneOf": [{ "const": "Red", "title": "Red" }, { "const": "Blue", "title": "Blue" }] }
        } } } })
        .to_string()
}

/// A fake agent that sends `ask` (a JSON line) when prompted and a
/// permission request 100 when `session/new` succeeds if `permission` is
/// set.
fn asking_agent(log: &Path, ask: &str, permission: bool) -> Command {
    let quote = |line: &str| line.replace('\'', r"'\''");
    let permission_line = if permission {
        r#"echo '{"jsonrpc":"2.0","id":100,"method":"session/request_permission","params":{"toolCall":{"toolCallId":"tc1"},"options":[{"optionId":"allow","name":"Allow","kind":"allow_once"}]}}'"#
    }
    else {
        ":"
    };
    let script = format!(
                         r#"while IFS= read -r line; do
          printf '%s\n' "$line" >> '{log}'
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":1,\"agentCapabilities\":{{}}}}}}" ;;
            session/new) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"sess-1\"}}}}"; {permission_line} ;;
            session/prompt) echo '{ask}' ;;
            session/set_config_option)
              echo '{{"jsonrpc":"2.0","method":"$/cancel_request","params":{{"requestId":200}}}}'
              echo '{{"jsonrpc":"2.0","method":"$/cancel_request","params":{{"requestId":100}}}}'
              echo '{{"jsonrpc":"2.0","method":"$/cancel_request","params":{{"requestId":999}}}}'
              echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#,
                         log = log.display(),
                         ask = quote(ask)
    );
    let mut command = Command::new("sh");
    command.arg("-c").arg(script);
    command
}

/// The lines the agent received, parsed.
fn received(log: &Path) -> Vec<Value> {
    std::fs::read_to_string(log).unwrap_or_default()
                                .lines()
                                .filter_map(|line| serde_json::from_str(line).ok())
                                .collect()
}

/// The client's replies to request `id`, once at least one has arrived.
async fn replies_to(log: &Path, id: i64) -> Vec<Value> {
    for _ in 0..100 {
        let replies: Vec<Value> = received(log).into_iter()
                                               .filter(|line| {
                                                   line.get("id") == Some(&json!(id))
                                                   && line.get("method").is_none()
                                               })
                                               .collect();
        if !replies.is_empty() {
            // Give a duplicate reply the chance to arrive too.
            tokio::time::sleep(Duration::from_millis(100)).await;
            return received(log).into_iter()
                                .filter(|line| {
                                    line.get("id") == Some(&json!(id))
                                    && line.get("method").is_none()
                                })
                                .collect();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("no reply to request {id}: {:?}", received(log));
}

async fn next_event(events: &mut mpsc::UnboundedReceiver<SessionEvent>) -> SessionEvent {
    tokio::time::timeout(Duration::from_secs(5), events.recv()).await
                                                               .expect("an event arrives")
                                                               .expect("the stream is open")
}

/// Connects, opens `sess-1`, prompts (in the background) and returns the
/// question the agent asked.
async fn asked(log: &Path) -> (AcpClient, mpsc::UnboundedReceiver<SessionEvent>, QuestionRequest) {
    let (client, mut events) =
        AcpClient::connect(asking_agent(log, &question_line("sess-1"), false)).await
                                                                              .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session");
    let prompting = client.clone();
    tokio::spawn(async move { prompting.session_prompt("sess-1", "ask me").await });
    match next_event(&mut events).await {
        SessionEvent::Question(request) => (client, events, request),
        other => panic!("expected a question, got {other:?}"),
    }
}

#[tokio::test]
async fn initialize_advertises_form_elicitation_only() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let _client = AcpClient::connect(asking_agent(log.path(), "{}", false)).await
                                                                           .expect("connect");
    let initialize = received(log.path()).into_iter()
                                         .find(|line| line["method"] == "initialize")
                                         .expect("initialize sent");
    assert_eq!(initialize["params"]["clientCapabilities"]["elicitation"],
               json!({ "form": {} }));
}

#[tokio::test]
async fn a_submitted_form_is_accepted_with_its_answers() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, mut events, request) = asked(log.path()).await;
    assert_eq!(request.tool_call_id.as_deref(), Some("toolu_1"));
    assert_eq!(request.fields[0].key, "question_0");

    client.answer_question(&request,
                           FormAnswer::Submitted(vec![("question_0".to_owned(),
                                                       AnswerValue::Text("Blue".to_owned()))]));

    assert!(matches!(next_event(&mut events).await,
                     SessionEvent::InputResolved { rpc_id } if rpc_id == json!(200)));
    let replies = replies_to(log.path(), 200).await;
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0]["result"],
               json!({ "action": "accept", "content": { "question_0": "Blue" } }));
}

#[tokio::test]
async fn stop_cancels_a_pending_form() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, mut events, _request) = asked(log.path()).await;

    client.session_cancel("sess-1").await.expect("cancel sent");

    assert!(matches!(next_event(&mut events).await,
                     SessionEvent::InputResolved { .. }));
    assert_eq!(replies_to(log.path(), 200).await[0]["result"],
               json!({ "action": "cancel" }));
}

#[tokio::test]
async fn a_withdrawn_form_is_answered_request_cancelled_and_a_late_answer_is_dropped() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, mut events, request) = asked(log.path()).await;

    client.session_set_config_option("sess-1", "mode", "x")
          .await
          .expect("cue sent");

    assert!(matches!(next_event(&mut events).await,
                     SessionEvent::InputResolved { rpc_id } if rpc_id == json!(200)));
    client.answer_question(&request, FormAnswer::Skipped);
    let replies = replies_to(log.path(), 200).await;
    assert_eq!(replies.len(), 1, "one reply only: {replies:?}");
    assert_eq!(replies[0]["error"]["code"], json!(-32800));
}

#[tokio::test]
async fn a_withdrawn_permission_request_is_answered_request_cancelled() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, mut events) =
        AcpClient::connect(asking_agent(log.path(), "{}", true)).await
                                                                .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session");
    assert!(matches!(next_event(&mut events).await,
                     SessionEvent::PermissionRequest(_)));

    client.session_set_config_option("sess-1", "mode", "x")
          .await
          .expect("cue sent");

    assert!(matches!(next_event(&mut events).await,
                     SessionEvent::InputResolved { rpc_id } if rpc_id == json!(100)));
    assert_eq!(replies_to(log.path(), 100).await[0]["error"]["code"],
               json!(-32800));
    // The cue also withdrew 200 and 999, neither pending: nothing for them.
    assert!(received(log.path()).iter()
                                .all(|line| line.get("id") != Some(&json!(999))));
}

#[tokio::test]
async fn a_form_for_an_unknown_session_is_refused_without_an_event() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, mut events) = AcpClient::connect(asking_agent(log.path(),
                                                               &question_line("someone-else"),
                                                               false)).await
                                                                      .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session");
    let prompting = client.clone();
    tokio::spawn(async move { prompting.session_prompt("sess-1", "ask me").await });

    assert_eq!(replies_to(log.path(), 200).await[0]["error"]["code"],
               json!(-32602));
    assert!(tokio::time::timeout(Duration::from_millis(200), events.recv()).await
                                                                           .is_err(),
            "no event is surfaced");
}

#[tokio::test]
async fn a_url_mode_elicitation_is_refused_without_an_event() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let url = json!({ "jsonrpc": "2.0", "id": 200, "method": "elicitation/create",
                      "params": { "mode": "url", "sessionId": "sess-1", "url": "https://example.com",
                                  "elicitationId": "e1", "message": "Sign in" } }).to_string();
    let (client, mut events) =
        AcpClient::connect(asking_agent(log.path(), &url, false)).await
                                                                 .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session");
    let prompting = client.clone();
    tokio::spawn(async move { prompting.session_prompt("sess-1", "ask me").await });

    assert_eq!(replies_to(log.path(), 200).await[0]["error"]["code"],
               json!(-32602));
    assert!(tokio::time::timeout(Duration::from_millis(200), events.recv()).await
                                                                           .is_err());
}

#[tokio::test]
async fn the_session_ending_cancels_a_pending_form() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, mut events, _request) = asked(log.path()).await;

    client.close().await;

    // The adapter is gone, so the reply cannot be read back; the resolution
    // is what takes the card off screen. It comes from the task waiting on
    // the form, so it may land either side of `Ended`.
    let resolved = |seen: &[SessionEvent]| {
        seen.iter()
            .any(|event| matches!(event, SessionEvent::InputResolved { .. }))
    };
    let ended = |seen: &[SessionEvent]| {
        seen.iter()
            .any(|event| matches!(event, SessionEvent::Ended(_)))
    };
    let mut seen = Vec::new();
    while !(resolved(&seen) && ended(&seen)) {
        match tokio::time::timeout(Duration::from_secs(5), events.recv()).await {
            Ok(Some(event)) => seen.push(event),
            Ok(None) | Err(_) => break,
        }
    }
    assert!(seen.iter()
                .any(|event| matches!(event, SessionEvent::InputResolved { .. })),
            "the form is resolved: {seen:?}");
}

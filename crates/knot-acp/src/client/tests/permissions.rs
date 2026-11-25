//! `session/request_permission`: delivering the user's decision, answering
//! a pending request on cancel and on close, and choosing the option of the
//! kind the user picked.

use super::*;

/// Fake agent that, once `session/new` succeeds, streams a text delta
/// and a turn-end update, then sends a `session/request_permission`
/// request; on receiving the client's response it echoes the chosen
/// option back as one more text delta, so the test can assert the
/// decision actually reached the agent.
fn permission_flow_agent() -> Command {
    permission_agent(r#"[{\"optionId\":\"allow-once\",\"name\":\"Allow\"},{\"optionId\":\"deny\",\"name\":\"Deny\"}]"#)
}

/// [`permission_flow_agent`], offering `options` - JSON with its quotes
/// escaped for the shell's `echo "..."`.
fn permission_agent(options: &str) -> Command {
    sh_agent(
             r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
              ;;
            session/new)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}"
              echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"hello\"}}"
              echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"turn_end\",\"stopReason\":\"end_turn\"}}"
              echo "{\"jsonrpc\":\"2.0\",\"id\":100,\"method\":\"session/request_permission\",\"params\":{\"toolCall\":{\"toolCallId\":\"tc1\"},\"options\":__OPTIONS__}}"
              ;;
            session/cancel)
              echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"cancel-received\"}}"
              ;;
            "")
              pid=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              if [ "$pid" = "100" ]; then
                if echo "$line" | grep -q '"outcome":"cancelled"'; then
                  opt=cancelled
                else
                  opt=$(echo "$line" | sed -E 's/.*"optionId":"([^"]+)".*/\1/')
                fi
                echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"decision:$opt\"}}"
              fi
              ;;
          esac
        done"#.replace("__OPTIONS__", options),
    )
}

#[tokio::test]
async fn close_while_permission_pending_ends_session_without_hanging() {
    let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                          .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session id");
    let _text = events.recv().await.expect("text delta");
    let _turn_end = events.recv().await.expect("turn end");
    let _permission = events.recv().await.expect("permission request");

    client.close().await;

    // Per the "session closed while a permission request is pending"
    // scenario, the still-pending decision auto-resolves to Deny
    // (rather than hanging) as part of tearing the session down.
    let ended = events.recv().await.expect("ended event");
    assert!(matches!(ended, SessionEvent::Ended(_)));
}

#[tokio::test]
async fn ordered_updates_stream_text_and_turn_end() {
    let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                          .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session id");

    let first = events.recv().await.expect("first update");
    let second = events.recv().await.expect("second update");

    assert!(matches!(first, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "hello"));
    assert!(matches!(second, SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason }) if stop_reason.as_str() == "end_turn"));
}

/// The next event, which has to be the report that the request with
/// `rpc_id` is no longer pending (#344). It precedes the agent's own reply:
/// the client says the request is resolved before it answers it.
async fn expect_resolved(events: &mut tokio::sync::mpsc::UnboundedReceiver<SessionEvent>,
                         rpc_id: &Value) {
    let event = tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
        .await
        .expect("the resolution is reported")
        .expect("an event");
    assert!(matches!(&event, SessionEvent::InputResolved { rpc_id: resolved } if resolved == rpc_id),
            "expected {rpc_id} resolved, got {event:?}");
}

#[tokio::test]
async fn permission_deny_decision_is_delivered_to_the_agent() {
    let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                          .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session id");
    let _text = events.recv().await.expect("text delta");
    let _turn_end = events.recv().await.expect("turn end");
    let permission = match events.recv().await.expect("permission request") {
        SessionEvent::PermissionRequest(request) => request,
        other => panic!("expected a permission request, got {other:?}"),
    };

    client.answer_permission(&permission, PermissionDecision::Deny);

    expect_resolved(&mut events, &permission.rpc_id).await;
    let confirmation = events.recv().await.expect("decision echoed back");
    assert!(matches!(confirmation, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "decision:deny"));
}

/// Cancelling a turn answers the permission request the agent is blocked
/// on with the `cancelled` outcome, after sending `session/cancel` - the
/// client's half of cancellation per
/// <https://agentclientprotocol.com/protocol/prompt-turn>, "Cancellation".
/// Without it the agent waited on the request and Stop did nothing (#267).
#[tokio::test]
async fn cancelling_answers_a_pending_permission_request_as_cancelled() {
    let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                          .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session id");
    let _text = events.recv().await.expect("text delta");
    let _turn_end = events.recv().await.expect("turn end");
    let permission = match events.recv().await.expect("permission request") {
        SessionEvent::PermissionRequest(request) => request,
        other => panic!("expected a permission request, got {other:?}"),
    };

    client.session_cancel("sess-1").await.expect("cancel sent");

    // Stop answers the request without the UI ever touching it, so this
    // report is the only thing that can take it off screen (#344). It can
    // land either side of the agent's acknowledgement of the cancel.
    let mut seen = Vec::new();
    while seen.len() < 3 {
        seen.push(tokio::time::timeout(std::time::Duration::from_secs(5), events.recv())
            .await
            .expect("the cancel's events arrive")
            .expect("an event"));
    }
    assert!(seen.iter().any(|event| matches!(event,
                SessionEvent::InputResolved { rpc_id } if *rpc_id == permission.rpc_id)),
            "the cancelled request is reported resolved: {seen:?}");
    let texts: Vec<_> =
        seen.iter()
            .filter_map(|event| match event {
                SessionEvent::Update(SessionUpdate::TextDelta { text }) => Some(text.as_str()),
                _ => None,
            })
            .collect();
    assert_eq!(texts, ["cancel-received", "decision:cancelled"]);
}

/// Claude's adapter lists Always Allow first. A plain Allow must still
/// answer with the allow-once option, and Always Allow with its own (#530).
#[tokio::test]
async fn allow_and_always_allow_answer_with_their_own_kinds() {
    let claude = r#"[{\"kind\":\"allow_always\",\"name\":\"Always Allow\",\"optionId\":\"allow_always\"},{\"kind\":\"allow_once\",\"name\":\"Allow\",\"optionId\":\"allow\"},{\"kind\":\"reject_once\",\"name\":\"Reject\",\"optionId\":\"reject\"}]"#;
    for (decision, expected) in [(PermissionDecision::Allow, "decision:allow"),
                                 (PermissionDecision::AllowAlways, "decision:allow_always"),
                                 (PermissionDecision::Deny, "decision:reject")]
    {
        let (client, mut events) = AcpClient::connect(permission_agent(claude)).await
                                                                               .expect("connect");
        client.session_new("/tmp/project", None)
              .await
              .expect("session id");
        let _text = events.recv().await.expect("text delta");
        let _turn_end = events.recv().await.expect("turn end");
        let permission = match events.recv().await.expect("permission request") {
            SessionEvent::PermissionRequest(request) => request,
            other => panic!("expected a permission request, got {other:?}"),
        };
        assert!(permission.has_kinds(), "the kinds survived parsing");

        client.answer_permission(&permission, decision);

        expect_resolved(&mut events, &permission.rpc_id).await;
        let confirmation = events.recv().await.expect("decision echoed back");
        assert!(matches!(&confirmation,
                         SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == expected),
                "{decision:?} answered {confirmation:?}");
    }
}

//! Unit tests for [`super`].

use super::*;
use crate::error::SessionEndCause;

mod config_options;
mod permissions;

/// A fake agent running `script` under `sh`.
///
/// Six fixtures built this the same way and differed only in the script,
/// which is the only part worth reading in any of them.
fn sh_agent(script: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new("sh");
    command.arg("-c").arg(script);
    command
}

/// A fake agent handling `initialize` (with a configurable protocol
/// version and resume capability) and `session/new`/`session/load`.
fn fake_agent(protocol_version: u32, supports_resume: bool) -> Command {
    let script = format!(
                         r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{protocol_version},\"agentCapabilities\":{{\"loadSession\":{supports_resume}}}}}}}" ;;
            session/new) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"sess-1\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#
    );
    sh_agent(script)
}

/// A fake agent whose `session/prompt` answers with a stop reason and
/// sends no `turn_end` notification - which is how ACP actually ends a
/// turn.
fn prompting_agent() -> Command {
    let script = format!(
                         r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{}}}}}}" ;;
            session/prompt) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"stopReason\":\"end_turn\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#
    );
    sh_agent(script)
}

/// The turn-end event has to be synthesized from the `session/prompt`
/// response, because ACP has no `turn_end` session update. Without it a
/// caller that gates input on "a turn is in flight" - the panel's Send
/// button and Enter key both do - stays blocked forever after the very
/// first prompt.
#[tokio::test]
async fn a_finished_prompt_emits_a_turn_end_with_the_responses_stop_reason() {
    let (client, mut events) = AcpClient::connect(prompting_agent()).await
                                                                    .expect("connect");

    client.session_prompt("sess-1", "hello")
          .await
          .expect("prompt");

    let event = events.recv().await.expect("a turn-end event");
    assert!(matches!(&event,
                     SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason })
                     if stop_reason.as_str() == "end_turn"),
            "expected a turn end, got {event:?}");
}

/// A prompt that fails must still end the turn, or the same gate wedges
/// the panel permanently on one bad request.
#[tokio::test]
async fn a_failed_prompt_still_ends_the_turn() {
    let (client, mut events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                              .expect("connect");
    drop(client.close());

    let _ = client.session_prompt("sess-1", "hello").await;

    let event = events.recv().await.expect("a turn-end event");
    assert!(matches!(&event, SessionEvent::Update(SessionUpdate::TurnEnd { .. })),
            "expected a turn end even on failure, got {event:?}");
}

#[tokio::test]
async fn connect_negotiates_matching_protocol_version() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                              .expect("connect");

    assert!(client.capabilities().supports_resume);
}

#[tokio::test]
async fn connect_fails_closed_on_unsupported_protocol_version() {
    let result = AcpClient::connect(fake_agent(PROTOCOL_VERSION + 1, false)).await;

    assert!(matches!(
                result,
                Err(AcpError::UnsupportedProtocolVersion { agent, client }) if agent == PROTOCOL_VERSION + 1 && client == PROTOCOL_VERSION
            ));
}

#[tokio::test]
async fn session_new_returns_session_id() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                              .expect("connect");

    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session id");

    assert_eq!(session.session_id, "sess-1");
}

/// A fake agent that appends every line it receives to `log_path`, so
/// the test can inspect the raw `session/new`/`session/load` params it
/// was sent. `declares_http` controls whether its `initialize`
/// response advertises `mcpCapabilities.http` - required before an
/// `http`-type `mcpServers` entry is valid per the ACP spec.
fn logging_fake_agent(log_path: &std::path::Path, declares_http: bool) -> Command {
    let mcp_capabilities = if declares_http {
        r#"{\"http\":true}"#
    }
    else {
        r#"{}"#
    };
    let script = format!(
                         r#"while IFS= read -r line; do
          echo "$line" >> '{}'
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{\"loadSession\":true,\"mcpCapabilities\":{mcp_capabilities}}}}}}}" ;;
            session/new) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"sess-1\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#,
                         log_path.display()
    );
    sh_agent(script)
}

#[tokio::test]
async fn session_new_carries_the_knot_mcp_server_when_enabled_and_supported() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), true)).await
                                                                .expect("connect");
    client.session_new("/tmp/project", Some(endpoint(None)))
          .await
          .expect("session");

    let log = std::fs::read_to_string(log.path()).unwrap();
    let request_line = log.lines()
                          .find(|line| line.contains("session/new"))
                          .expect("session/new request logged");
    assert!(request_line.contains(r#""type":"http""#));
    assert!(request_line.contains(r#""name":"knot""#));
    assert!(request_line.contains(r#""url":"http://127.0.0.1:8767/mcp""#));
    assert!(request_line.contains(r#""headers":[]"#));
}

fn endpoint(bearer: Option<&str>) -> McpEndpoint<'_> {
    McpEndpoint { url: "http://127.0.0.1:8767/mcp",
                  bearer }
}

/// `mcp-auth` D3: the agent's tool credential rides in the entry's
/// `headers`, as ACP's `HttpHeader { name, value }`, on both `session/new`
/// and `session/load` - and never in the URL.
#[tokio::test]
async fn the_tool_credential_rides_in_the_mcp_headers_on_new_and_load() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), true)).await
                                                                .expect("connect");
    let credential = Some("knot_a_test-credential");
    client.session_new("/tmp/project", Some(endpoint(credential)))
          .await
          .expect("session");
    client.session_load("sess-1", "/tmp/project", Some(endpoint(credential)))
          .await
          .expect("load");

    let log = std::fs::read_to_string(log.path()).unwrap();
    for method in ["session/new", "session/load"] {
        let request: Value = log.lines()
                                .find(|line| line.contains(method))
                                .and_then(|line| serde_json::from_str(line).ok())
                                .unwrap_or_else(|| panic!("{method} logged"));
        let server = &request["params"]["mcpServers"][0];
        assert_eq!(server["headers"],
                   json!([{ "name": "Authorization", "value": "Bearer knot_a_test-credential" }]),
                   "{method}");
        assert_eq!(server["url"], "http://127.0.0.1:8767/mcp", "{method}");
    }
    assert!(!format!("{:?}", endpoint(credential)).contains("knot_a_"),
            "Debug leaves the credential out");
}

#[tokio::test]
async fn session_new_omits_the_mcp_server_when_the_agent_does_not_support_http() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), false)).await
                                                                 .expect("connect");
    client.session_new("/tmp/project", Some(endpoint(Some("knot_a_unused"))))
          .await
          .expect("session");

    let log = std::fs::read_to_string(log.path()).unwrap();
    let request_line = log.lines()
                          .find(|line| line.contains("session/new"))
                          .expect("session/new request logged");
    assert!(request_line.contains(r#""mcpServers":[]"#));
    assert!(!request_line.contains("knot_a_"),
            "no credential goes to an adapter that takes no HTTP server");
}

/// The `session/new` request line `client` sent, as `logging_fake_agent`
/// logged it.
async fn logged_session_new(client: &AcpClient, log: &std::path::Path) -> Value {
    client.session_new("/tmp/project", None)
          .await
          .expect("session");
    let log = std::fs::read_to_string(log).unwrap();
    let line = log.lines()
                  .find(|line| line.contains("session/new"))
                  .expect("session/new request logged");
    serde_json::from_str(line).expect("a JSON request")
}

#[tokio::test]
async fn session_new_carries_the_session_meta() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let meta = json!({ "claudeCode": { "options": { "extraArgs": { "remote-control": null } } } });
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), false)).await
                                                                 .expect("connect");
    let client = client.with_session_meta(Some(meta.clone()));

    let request = logged_session_new(&client, log.path()).await;
    assert_eq!(request["params"]["_meta"], meta);
    assert_eq!(request["params"]["cwd"], "/tmp/project");
}

#[tokio::test]
async fn session_new_sends_no_meta_without_one() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), false)).await
                                                                 .expect("connect");

    let request = logged_session_new(&client, log.path()).await;
    assert!(request["params"].get("_meta").is_none());
}

#[tokio::test]
async fn session_new_sends_no_mcp_servers_when_disabled() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), true)).await
                                                                .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session");

    let log = std::fs::read_to_string(log.path()).unwrap();
    let request_line = log.lines()
                          .find(|line| line.contains("session/new"))
                          .expect("session/new request logged");
    assert!(request_line.contains(r#""mcpServers":[]"#));
}

#[tokio::test]
async fn session_load_fails_closed_when_resume_unsupported() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, false)).await
                                                               .expect("connect");

    let result = client.session_load("sess-1", "/tmp/project", None).await;

    assert!(matches!(result, Err(AcpError::ResumeNotSupported)));
}

/// ACP's load response has no `sessionId`, and `codex-acp` sends none: the
/// session keeps the id it was loaded by. `fake_agent` answers `session/load`
/// with an empty result, which is that shape.
#[tokio::test]
async fn a_load_response_without_a_session_id_keeps_the_requested_one() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                              .expect("connect");

    let session = client.session_load("thread-7", "/tmp/project", None)
                        .await
                        .expect("an empty load result is a successful load");

    assert_eq!(session.session_id, "thread-7");
}

/// An adapter that does name the loaded session is taken at its word.
#[tokio::test]
async fn a_load_response_naming_a_session_id_is_used() {
    let script = format!(
                         r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{\"loadSession\":true}}}}}}" ;;
            session/load) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"renamed\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#
    );
    let (client, _events) = AcpClient::connect(sh_agent(script)).await.expect("connect");

    let session = client.session_load("thread-7", "/tmp/project", None)
                        .await
                        .expect("load");

    assert_eq!(session.session_id, "renamed");
}

#[tokio::test]
async fn subprocess_exit_ends_session_and_resolves_pending_request_with_error() {
    // A subprocess that answers `initialize` then exits immediately,
    // simulating a crash mid-turn: the next request must resolve with
    // an error rather than hang.
    let command = sh_agent(
                           r#"read -r line
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}""#,
    );
    let (client, mut events) = AcpClient::connect(command).await.expect("connect");

    let result = client.session_new("/tmp/project", None).await;

    assert!(result.is_err());
    let ended = events.recv().await.expect("ended event");
    assert!(matches!(ended,
                     SessionEvent::Ended(SessionEndCause::ProcessExited { .. })));
}

/// The panel agent's session root, reachable from the client because the
/// transport module is private. See
/// `openspec/specs/agent-processes/spec.md` -- "Every running agent exposes
/// a session root process".
#[tokio::test]
async fn a_connected_client_reports_the_adapters_pid() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, false)).await
                                                               .expect("connect");

    let pid = client.process_id()
                    .expect("a live adapter has a process id");

    assert!(pid > 1, "got {pid}");
    assert_ne!(pid, std::process::id());
}

/// A fake agent whose `initialize` reports `agentInfo`, in the shape of the
/// spec's example (<https://agentclientprotocol.com/protocol/initialization>).
fn identified_agent() -> Command {
    let script = format!(
                         r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{}},\"agentInfo\":{{\"name\":\"my-agent\",\"title\":\"My Agent\",\"version\":\"1.0.0\"}}}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#
    );
    sh_agent(script)
}

/// What the handshake reported is what the client hands back, so a session
/// can say which agent and version it is running.
#[tokio::test]
async fn connect_keeps_the_agent_info_initialize_reported() {
    let (client, _events) = AcpClient::connect(identified_agent()).await
                                                                  .expect("connect");

    let info = client.agent_info().expect("agentInfo was reported");
    assert_eq!(info.display_name(), "My Agent");
    assert_eq!(info.version(), Some("1.0.0"));
}

#[tokio::test]
async fn connect_reports_no_agent_info_when_the_agent_sends_none() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, false)).await
                                                               .expect("connect");

    assert!(client.agent_info().is_none());
}

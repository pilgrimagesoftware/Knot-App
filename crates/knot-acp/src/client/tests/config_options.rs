//! Session Config Options: parsing what `session/new` declares, with and
//! without a `category`, and `session/set_config_option`.

use super::*;

/// Fake agent declaring one `select` Session Config Option ("mode") on
/// `session/new`, and answering `session/set_config_option` with an
/// updated `currentValue`.
fn config_options_agent() -> Command {
    sh_agent(
             r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
              ;;
            session/new)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\",\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"category\":\"mode\",\"type\":\"select\",\"currentValue\":\"ask\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
              ;;
            session/set_config_option)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"category\":\"mode\",\"type\":\"select\",\"currentValue\":\"code\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
              ;;
            *) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}" ;;
          esac
        done"#,
    )
}

#[tokio::test]
async fn session_new_parses_declared_config_options() {
    let (client, _events) = AcpClient::connect(config_options_agent()).await
                                                                      .expect("connect");

    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session");

    assert_eq!(session.config_options.len(), 1);
    assert_eq!(session.config_options[0].id, "mode");
    assert_eq!(session.config_options[0].category.as_deref(), Some("mode"));
    assert_eq!(session.config_options[0].options.len(), 2);
}

/// The same fake agent, minus the `category` field - which ACP makes
/// optional: "Clients MUST handle missing or unknown categories
/// gracefully"
/// (<https://agentclientprotocol.com/protocol/v2/session-config-options>).
///
/// The categorized fixture above is the reason issue #194 went unnoticed
/// for so long: it hard-codes `"category":"mode"`, so every test saw a
/// roster Knot could match while real agents that omitted the field lost
/// all three selectors and the prompt's risk colour.
fn uncategorized_config_options_agent() -> Command {
    sh_agent(
             r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
              ;;
            session/new)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\",\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"type\":\"select\",\"currentValue\":\"ask\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
              ;;
            *) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}" ;;
          esac
        done"#,
    )
}

/// An agent that omits `category` must still parse, and must reach the
/// client with `category: None` rather than being dropped or defaulted to
/// something that would match by accident.
#[tokio::test]
async fn session_new_accepts_config_options_without_a_category() {
    let (client, _events) =
        AcpClient::connect(uncategorized_config_options_agent()).await
                                                                .expect("connect");

    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session");

    assert_eq!(session.config_options.len(), 1);
    assert_eq!(session.config_options[0].id, "mode");
    assert_eq!(session.config_options[0].category, None,
               "a missing category must stay missing, not acquire a default");
    assert_eq!(session.config_options[0].kind, "select");
    assert_eq!(session.config_options[0].options.len(), 2);
}

#[tokio::test]
async fn set_config_option_sends_the_selection_and_returns_the_updated_list() {
    let (client, _events) = AcpClient::connect(config_options_agent()).await
                                                                      .expect("connect");
    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session");

    let updated = client.session_set_config_option(&session.session_id, "mode", "code")
                        .await
                        .expect("set config option");

    assert_eq!(updated[0].current_value, serde_json::json!("code"));
}

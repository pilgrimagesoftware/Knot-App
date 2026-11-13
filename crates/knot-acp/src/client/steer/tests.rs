//! Steering against fake adapters whose `initialize` and `_session/steering`
//! replies take `claude-agent-acp` 0.101's shapes.

use tokio::process::Command;

use crate::client::AcpClient;
use crate::error::AcpError;
use crate::protocol::PROTOCOL_VERSION;
use crate::protocol::steering::SteerOutcome;

/// A fake agent that advertises steering when `advertises` and answers a
/// steer with `outcome`, logging each method to `$KNOT_TEST_LOG`.
fn steering_agent(advertises: bool, outcome: &str, log: &std::path::Path) -> Command {
    let meta = if advertises {
        r#",\"_meta\":{\"steering\":{\"supported\":true}}"#
    }
    else {
        ""
    };
    let script = format!(
                         r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          echo "$method" >> "{log}"
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{}}{meta}}}}}" ;;
            _session/steering) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"outcome\":\"{outcome}\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#,
                         log = log.display()
    );
    let mut command = Command::new("sh");
    command.arg("-c").arg(script);
    command
}

fn methods(log: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(log).unwrap_or_default()
                                .lines()
                                .map(str::to_owned)
                                .collect()
}

#[tokio::test]
async fn an_advertising_agent_takes_a_steer() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let log = dir.path().join("methods.log");
    let (client, _events) =
        AcpClient::connect(steering_agent(true, "injected", &log)).await
                                                                  .expect("connects");

    assert!(client.capabilities().supports_steering);
    assert_eq!(client.session_steer("s1", "also the tests")
                     .await
                     .expect("steers"),
               SteerOutcome::Injected);
    assert!(methods(&log).contains(&"_session/steering".to_owned()));
}

#[tokio::test]
async fn no_running_turn_hands_the_prompt_back() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let log = dir.path().join("methods.log");
    let (client, _events) =
        AcpClient::connect(steering_agent(true, "promptRequired", &log)).await
                                                                        .expect("connects");

    assert_eq!(client.session_steer("s1", "next").await.expect("steers"),
               SteerOutcome::PromptRequired);
}

/// Nothing is sent to an agent that did not say it can take a steer.
#[tokio::test]
async fn an_agent_without_steering_is_refused_locally() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let log = dir.path().join("methods.log");
    let (client, _events) =
        AcpClient::connect(steering_agent(false, "injected", &log)).await
                                                                   .expect("connects");

    assert!(!client.capabilities().supports_steering);
    assert!(matches!(client.session_steer("s1", "x").await,
                     Err(AcpError::SteeringNotSupported)));
    assert!(!methods(&log).contains(&"_session/steering".to_owned()));
}

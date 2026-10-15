//! Listing a real stdio server - a few lines of `sh` that speak just enough
//! MCP - so the process handling is exercised, not only the protocol.

#![cfg(unix)]

use std::time::{Duration, Instant};

use knot_mcp_client::{ClientError, Endpoint, StdioServer, list_tools};

/// Answers `initialize` and `tools/list`, ignoring the notification between.
const FAKE_SERVER: &str = r#"
while IFS= read -r line; do
  case "$line" in
    *'"initialize"'*) printf '%s\n' 'not json: a banner printed by mistake'
                      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fake","version":"0"}}}' ;;
    *'"tools/list"'*) printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"echo\",\"description\":\"Says $GREETING\"}]}}" ;;
  esac
done
"#;

fn sh(script: &str) -> Endpoint {
    Endpoint::Stdio(StdioServer { program: "sh".to_owned(),
                                  args:    vec!["-c".to_owned(), script.to_owned()],
                                  env:     vec![("GREETING".to_owned(), "hello".to_owned())],
                                  cwd:     None, })
}

fn path() -> String {
    std::env::var("PATH").unwrap_or_default()
}

#[test]
fn lists_a_stdio_servers_tools_with_its_environment() {
    let tools = list_tools(&sh(FAKE_SERVER), &path(), Duration::from_secs(10)).expect("listed");

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
    assert_eq!(tools[0].description.as_deref(),
               Some("Says hello"),
               "the configured environment reaches the server");
}

/// A server that never answers costs the timeout and no more: the whole
/// process group is stopped, so a grandchild cannot hold the pipe open.
#[test]
fn a_silent_server_times_out_and_is_stopped() {
    let silent = "sleep 30 & cat > /dev/null";
    let started = Instant::now();

    let error = list_tools(&sh(silent), &path(), Duration::from_millis(400)).expect_err("silent");

    assert!(matches!(error, ClientError::TimedOut), "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(5),
            "returned after {:?}",
            started.elapsed());
}

#[test]
fn a_program_that_is_not_installed_says_so() {
    let missing = Endpoint::Stdio(StdioServer { program: "knot-no-such-mcp-server".to_owned(),
                                                args:    Vec::new(),
                                                env:     Vec::new(),
                                                cwd:     None, });

    let error = list_tools(&missing, &path(), Duration::from_secs(5)).expect_err("missing");

    assert!(matches!(error, ClientError::Spawn { .. }), "{error:?}");
}

#[test]
fn a_server_that_exits_is_closed() {
    let error = list_tools(&sh("exit 0"), &path(), Duration::from_secs(5)).expect_err("exited");

    assert!(matches!(error, ClientError::Closed), "{error:?}");
}

//! Who may reach what (`mcp-auth` tasks 2.1-2.4, 4.3): the `Host` check,
//! every route against every kind of credential, the hook routes' agent
//! scope, and that refusals are logged and counted without the credential.

use std::time::Duration;

use knot_mcp::{CredentialRegistry, HookError, RefusalLog, RefusalSource, Secret};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

/// Accepts every hook, so a refusal can only come from the server's checks.
struct AcceptingHooks;

impl AgentHookHandler for AcceptingHooks {
    fn register(&self, _request: &HookRequest) -> Result<Value, HookError> {
        Ok(json!({ "ok": true }))
    }

    fn status(&self, _request: &HookRequest) -> Result<Value, HookError> {
        Ok(json!({ "ok": true }))
    }
}

struct Fixture {
    server:      McpServer,
    base:        String,
    port:        u16,
    credentials: CredentialRegistry,
    refusals:    RefusalLog,
}

async fn fixture() -> Fixture {
    let credentials = support::registry();
    let refusals = RefusalLog::new();
    let mut server = McpServer::new(0, Arc::new(EmptyCatalog), no_agents())
        .with_hook_handler(Arc::new(AcceptingHooks))
        .with_credentials(credentials.clone())
        .with_refusals(refusals.clone());
    server.start().await.expect("server starts");
    let addr = server.bound_addr().expect("bound address");
    Fixture { server,
              base: format!("http://{addr}"),
              port: addr.port(),
              credentials,
              refusals }
}

/// The status line's code for a raw HTTP/1.1 request with `host` as its
/// `Host` header (or none) - reqwest always derives `Host` from the URL.
async fn status_with_host(port: u16, path: &str, host: Option<&str>) -> u16 {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await
                                                                        .expect("connects");
    let host_line = host.map_or_else(String::new, |host| format!("Host: {host}\r\n"));
    let request = format!("GET {path} HTTP/1.1\r\n{host_line}Authorization: Bearer {}\r\n\
                           Connection: close\r\n\r\n",
                          support::TOKEN.expose());
    stream.write_all(request.as_bytes()).await.expect("writes");
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_string(&mut response)).await
                                                                                     .expect("answers in time")
                                                                                     .expect("reads");
    response.split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("a status line")
}

#[tokio::test]
async fn only_a_loopback_host_on_the_listening_port_is_served() {
    let mut fixture = fixture().await;
    let port = fixture.port;
    let other_port = port.wrapping_add(1).max(1);

    for (host, expected) in [(Some(format!("127.0.0.1:{port}")), 200),
                             (Some(format!("localhost:{port}")), 200),
                             (Some(format!("other.example:{port}")), 403),
                             (Some(format!("127.0.0.1:{other_port}")), 403),
                             (None, 403)]
    {
        for path in ["/health", "/"] {
            assert_eq!(status_with_host(port, path, host.as_deref()).await,
                       expected,
                       "{path} with Host {host:?}");
        }
    }
    fixture.server.stop();
}

/// One request to each authenticated route.
#[derive(Debug, Clone, Copy)]
enum Target {
    Info,
    McpPost,
    McpGet,
    StatusRead,
    StatusHook,
    RegisterHook,
}

const TARGETS: [Target; 6] = [Target::Info,
                              Target::McpPost,
                              Target::McpGet,
                              Target::StatusRead,
                              Target::StatusHook,
                              Target::RegisterHook];

fn hook_body(agent: Uuid) -> Value {
    json!({ "agent_id": agent.to_string(), "agent": "claude", "status": "running" })
}

async fn send(target: Target, base: &str, credential: Option<&str>, agent: Uuid)
              -> reqwest::Response {
    let client = reqwest::Client::new();
    let request = match target {
        Target::Info => client.get(base.to_owned()),
        Target::McpPost => {
            client.post(format!("{base}/mcp"))
                  .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
        }
        Target::McpGet => client.get(format!("{base}/mcp")),
        Target::StatusRead => client.get(format!("{base}/api/v1/agent/status")),
        Target::StatusHook => client.post(format!("{base}/api/v1/agent/status"))
                                    .json(&hook_body(agent)),
        Target::RegisterHook => client.post(format!("{base}/api/v1/agent/register"))
                                      .json(&hook_body(agent)),
    };
    let request = match credential {
        Some(credential) => request.bearer_auth(credential),
        None => request,
    };
    request.send().await.expect("the server answers")
}

#[tokio::test]
async fn without_a_known_credential_every_route_but_health_is_401() {
    let mut fixture = fixture().await;
    let agent = Uuid::new_v4();
    let unknown = Secret::generate(knot_mcp::CredentialKind::Client);

    for target in TARGETS {
        for credential in [None, Some(unknown.expose()), Some("not-a-credential")] {
            let response = send(target, &fixture.base, credential, agent).await;
            assert_eq!(response.status(), 401, "{target:?} with {credential:?}");
            assert_eq!(response.headers()["www-authenticate"],
                       "Bearer realm=\"knot\"",
                       "{target:?}");
            let body: Value = response.json().await.expect("a JSON-RPC error");
            assert!(body["id"].is_null() && body["error"].is_object(),
                    "{target:?}");
        }
    }
    assert_eq!(fixture.server.live_sessions(),
               0,
               "no refused request opened a session");

    let health = reqwest::get(format!("{}/health", fixture.base)).await
                                                                 .expect("health answers");
    assert_eq!(health.status(), 200);
    assert_eq!(health.text().await.unwrap(), "OK");
    fixture.server.stop();
}

#[tokio::test]
async fn each_credential_reaches_exactly_its_routes() {
    let mut fixture = fixture().await;
    let agent = Uuid::new_v4();
    let pair = fixture.credentials.issue_agent_pair(agent);
    // The client token's hooks name an agent with no live connection of its
    // own: for one that has, D6 refuses them (tested below).
    let unheld = Uuid::new_v4();

    // (target, tool credential, hook credential, client token): served?
    let expected = [(Target::Info, false, false, true),
                    (Target::McpPost, true, false, true),
                    (Target::McpGet, true, false, true),
                    (Target::StatusRead, true, false, true),
                    (Target::StatusHook, false, true, true),
                    (Target::RegisterHook, false, true, true)];
    for (target, tool, hooks, client) in expected {
        for (credential, named, served) in [(pair.tool.expose(), agent, tool),
                                            (pair.hooks.expose(), agent, hooks),
                                            (support::TOKEN.expose(), unheld, client)]
        {
            let status = send(target, &fixture.base, Some(credential), named).await
                                                                             .status();
            if served {
                assert!(status.is_success(), "{target:?}: {status}");
            }
            else {
                assert_eq!(status, 403, "{target:?}");
            }
        }
    }
    fixture.server.stop();
}

#[tokio::test]
async fn a_hook_credential_reports_only_for_its_own_agent() {
    let mut fixture = fixture().await;
    let (agent, other) = (Uuid::new_v4(), Uuid::new_v4());
    let pair = fixture.credentials.issue_agent_pair(agent);

    for target in [Target::StatusHook, Target::RegisterHook] {
        let own = send(target, &fixture.base, Some(pair.hooks.expose()), agent).await;
        assert!(own.status().is_success(), "{target:?}");
        let theirs = send(target, &fixture.base, Some(pair.hooks.expose()), other).await;
        assert_eq!(theirs.status(), 403, "{target:?}");
    }
    fixture.server.stop();
}

#[tokio::test]
async fn the_client_token_cannot_report_for_an_agent_its_credential_holds() {
    let mut fixture = fixture().await;
    let (held, free) = (Uuid::new_v4(), Uuid::new_v4());
    let pair = fixture.credentials.issue_agent_pair(held);
    // The agent's own connection is live.
    send(Target::McpPost,
         &fixture.base,
         Some(pair.tool.expose()),
         held).await;

    let token = support::TOKEN.expose();
    assert_eq!(send(Target::StatusHook, &fixture.base, Some(token), held).await
                                                                         .status(),
               403);
    assert!(send(Target::StatusHook, &fixture.base, Some(token), free).await
                                                                      .status()
                                                                      .is_success());
    fixture.server.stop();
}

/// D5/D6: holding an issued pair is what makes an agent off limits to the
/// client token - before its adapter's first request, and after its
/// connection has gone.
#[tokio::test]
async fn the_client_token_cannot_report_for_a_launched_agent_before_it_connects() {
    let mut fixture = fixture().await;
    let launched = Uuid::new_v4();
    fixture.credentials.issue_agent_pair(launched);
    let token = support::TOKEN.expose();

    for target in [Target::StatusHook, Target::RegisterHook] {
        assert_eq!(send(target, &fixture.base, Some(token), launched).await
                                                                     .status(),
                   403,
                   "{target:?} before the agent connected");
    }
    fixture.credentials.revoke_agent(launched);
    assert!(send(Target::StatusHook, &fixture.base, Some(token), launched).await
                                                                          .status()
                                                                          .is_success(),
            "once its adapter is gone the agent is an ordinary one again");
    fixture.server.stop();
}

#[tokio::test]
async fn a_rotated_credential_is_refused() {
    let mut fixture = fixture().await;
    let agent = Uuid::new_v4();
    let old = fixture.credentials.issue_agent_pair(agent);
    let new = fixture.credentials.issue_agent_pair(agent);

    for target in [Target::McpPost, Target::StatusRead] {
        assert_eq!(send(target, &fixture.base, Some(old.tool.expose()), agent).await
                                                                              .status(),
                   401,
                   "{target:?}");
        assert!(send(target, &fixture.base, Some(new.tool.expose()), agent).await
                                                                           .status()
                                                                           .is_success());
    }
    assert_eq!(send(Target::StatusHook,
                    &fixture.base,
                    Some(old.hooks.expose()),
                    agent).await
                          .status(),
               401);
    fixture.credentials.revoke_agent(agent);
    assert_eq!(send(Target::McpPost,
                    &fixture.base,
                    Some(new.tool.expose()),
                    agent).await
                          .status(),
               401,
               "a closed agent's credential ends with it");
    fixture.server.stop();
}

#[tokio::test]
async fn refusals_are_counted_against_the_agent_the_url_names() {
    let mut fixture = fixture().await;
    let agent = Uuid::new_v4();
    fixture.credentials.issue_agent_pair(agent);
    let client = reqwest::Client::builder().user_agent("probe-client/1.0")
                                           .build()
                                           .unwrap();
    for url in [format!("{}/mcp?agent={agent}", fixture.base),
                format!("{}/mcp", fixture.base)]
    {
        client.post(url)
              .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
              .send()
              .await
              .unwrap();
    }

    assert_eq!(fixture.refusals.refused_for(agent), 1);
    assert!(fixture.refusals
                   .snapshot()
                   .contains(&(RefusalSource::UserAgent("probe-client/1.0".into()), 1)));
    fixture.server.stop();
}

/// Hints come from unauthenticated URLs, so only a launched agent is counted
/// by name: a stream of made-up ids does not grow the table.
#[tokio::test]
async fn made_up_hints_do_not_grow_the_refusal_table() {
    let mut fixture = fixture().await;
    let client = reqwest::Client::new();
    for _ in 0..200 {
        client.post(format!("{}/mcp?agent={}", fixture.base, Uuid::new_v4()))
              .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
              .send()
              .await
              .unwrap();
    }

    assert_eq!(fixture.refusals.total(), 200);
    assert!(fixture.refusals.snapshot().len() <= 2,
            "{:?}",
            fixture.refusals.snapshot());
    fixture.server.stop();
}

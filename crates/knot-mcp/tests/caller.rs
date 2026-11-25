//! The connection a tool call arrives on reaches the catalog as a `Caller`
//! bound to the agent whose credential it presents (#539; `mcp-auth` D5).
//! The URL's `?agent=` is a hint: it must agree with the credential, and it
//! binds nothing on its own.

use std::sync::Arc;

use async_trait::async_trait;
use knot_agents::Agent;
use knot_mcp::{
    Caller, CredentialRegistry, McpServer, ToolCallResult, ToolCatalog, ToolDefinition,
};
use parking_lot::Mutex;
use serde_json::{Value, json};
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

/// Answers every call with the agent its caller is bound to, or `none`.
struct WhoCalls;

#[async_trait]
impl ToolCatalog for WhoCalls {
    fn list(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    async fn call(&self, _name: &str, _arguments: Value) -> ToolCallResult {
        ToolCallResult::ok("unreached")
    }

    async fn call_as(&self, _name: &str, _arguments: Value, caller: &Caller) -> ToolCallResult {
        ToolCallResult::ok(caller.agent
                                 .map_or_else(|| "none".to_owned(), |id| id.to_string()))
    }
}

async fn start() -> (McpServer, String, CredentialRegistry) {
    let no_agents: Arc<dyn Fn() -> Vec<Agent> + Send + Sync> = Arc::new(Vec::new);
    let credentials = support::registry();
    let mut server =
        McpServer::new(0, Arc::new(WhoCalls), no_agents).with_credentials(credentials.clone());
    server.start().await.expect("server starts");
    let addr = server.bound_addr().expect("bound address");
    (server, format!("http://{addr}/mcp"), credentials)
}

fn initialize() -> Value {
    json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" })
}

fn any_call() -> Value {
    json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "any", "arguments": {} } })
}

/// Makes one tool call at `url` on `session` with `client`, and returns who
/// the catalog saw.
async fn call_seen(client: &reqwest::Client, url: &str, session: &str) -> String {
    let call = client.post(url)
                     .header("Mcp-Session-Id", session)
                     .json(&any_call())
                     .send()
                     .await
                     .unwrap();
    let body: Value = call.json().await.unwrap();
    body["result"]["content"][0]["text"].as_str()
                                        .unwrap()
                                        .to_owned()
}

/// Initializes at `url` with `client`, returning the session it got.
async fn session_of(client: &reqwest::Client, url: &str) -> String {
    let init = client.post(url).json(&initialize()).send().await.unwrap();
    init.headers()["Mcp-Session-Id"].to_str()
                                    .unwrap()
                                    .to_owned()
}

/// Initializes at `url`, then makes one tool call on the session it got,
/// and returns who the catalog saw.
async fn caller_seen(client: &reqwest::Client, url: &str) -> String {
    let session = session_of(client, url).await;
    call_seen(client, url, &session).await
}

#[tokio::test]
async fn an_agents_credential_binds_its_connection() {
    let (mut server, url, credentials) = start().await;
    let agent = Uuid::new_v4();
    let client = support::client_with(&credentials.issue_agent_pair(agent).tool);

    assert_eq!(caller_seen(&client, &format!("{url}?agent={agent}")).await,
               agent.to_string());
    assert_eq!(caller_seen(&client, &url).await,
               agent.to_string(),
               "the hint is optional; the credential decides");
    server.stop();
}

#[tokio::test]
async fn the_client_token_leaves_the_connection_unbound_whatever_the_url_says() {
    let (mut server, url, _) = start().await;
    let client = support::client();
    assert_eq!(caller_seen(&client, &url).await, "none");
    assert_eq!(caller_seen(&client, &format!("{url}?agent={}", Uuid::new_v4())).await,
               "none");
    assert_eq!(caller_seen(&client, &format!("{url}?agent=not-a-uuid")).await,
               "none");
    server.stop();
}

#[tokio::test]
async fn a_hint_naming_another_agent_is_refused_before_any_session() {
    let (mut server, url, credentials) = start().await;
    let (agent, other) = (Uuid::new_v4(), Uuid::new_v4());
    let client = support::client_with(&credentials.issue_agent_pair(agent).tool);

    let response = client.post(format!("{url}?agent={other}"))
                         .json(&initialize())
                         .send()
                         .await
                         .unwrap();

    assert_eq!(response.status(), 403);
    assert!(response.headers().get("Mcp-Session-Id").is_none());
    assert_eq!(server.live_sessions(), 0, "no session was created");
    server.stop();
}

#[tokio::test]
async fn another_principals_session_id_does_not_carry_its_agent() {
    let (mut server, url, credentials) = start().await;
    let agent = Uuid::new_v4();
    let owner = support::client_with(&credentials.issue_agent_pair(agent).tool);
    let session = session_of(&owner, &url).await;

    assert_eq!(call_seen(&support::client(), &url, &session).await,
               "none",
               "the client token presenting an agent's session id acts as nobody");
    let intruder = support::client_with(&credentials.issue_agent_pair(Uuid::new_v4()).tool);
    assert_ne!(call_seen(&intruder, &url, &session).await,
               agent.to_string());
    assert_eq!(call_seen(&owner, &url, &session).await,
               agent.to_string(),
               "the agent's own session is untouched");
    server.stop();
}

/// Records every agent the server reports connected.
#[derive(Default)]
struct Connections(Mutex<Vec<Uuid>>);

#[async_trait]
impl ToolCatalog for Connections {
    fn list(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    async fn call(&self, _name: &str, _arguments: Value) -> ToolCallResult {
        ToolCallResult::ok("")
    }

    fn connected(&self, agent_id: Uuid) {
        self.0.lock().push(agent_id);
    }
}

/// #552: the server reports a connection bound by an agent's credential as
/// soon as it initializes, before any tool call - a resumed agent that never
/// calls one is still registered. The client token reports nobody, whatever
/// its URL names.
#[tokio::test]
async fn a_bound_connection_is_reported_on_initialize() {
    let catalog = Arc::new(Connections::default());
    let no_agents: Arc<dyn Fn() -> Vec<Agent> + Send + Sync> = Arc::new(Vec::new);
    let credentials = support::registry();
    let mut server =
        McpServer::new(0, catalog.clone(), no_agents).with_credentials(credentials.clone());
    server.start().await.expect("server starts");
    let url = format!("http://{}/mcp", server.bound_addr().expect("bound address"));
    let (agent, named) = (Uuid::new_v4(), Uuid::new_v4());
    let agent_client = support::client_with(&credentials.issue_agent_pair(agent).tool);

    for (client, target) in [(agent_client, format!("{url}?agent={agent}")),
                             (support::client(), format!("{url}?agent={named}")),
                             (support::client(), url.clone())]
    {
        client.post(&target)
              .json(&initialize())
              .send()
              .await
              .unwrap();
    }
    assert_eq!(*catalog.0.lock(),
               vec![agent],
               "an unbound connection reports nobody");
    server.stop();
}

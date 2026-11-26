//! MCP protocol versions over HTTP: the sessionless probe before
//! `initialize` (#431), version negotiation and the version header (#442).

use super::*;

/// #431: agy's MCP client (Go, `Mcp-Protocol-Version: 2026-07-28`) sends
/// `server/discover` before `initialize`, then `initialize` with whatever
/// session id the first answer carried. A server that hands out an id on
/// that sessionless probe and a different one on `initialize` makes the
/// client abandon the connection before `tools/list`. The sequence here is
/// the one captured from agy 1.5.9.
#[tokio::test]
async fn a_sessionless_probe_before_initialize_is_given_no_session() {
    let (mut server, base) = start_server(Arc::new(EmptyCatalog)).await;
    let client = support::client();

    let discover = client.post(format!("{base}/mcp"))
                         .header("Mcp-Protocol-Version", "2026-07-28")
                         .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover",
                                        "params": {} }))
                         .send()
                         .await
                         .unwrap();
    assert!(discover.headers().get("Mcp-Session-Id").is_none(),
            "no session before initialize");
    let body: Value = discover.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32601, "the probe is still answered");

    let init = client.post(format!("{base}/mcp"))
                     .json(&json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize",
                                    "params": { "protocolVersion": "2025-11-25" } }))
                     .send()
                     .await
                     .unwrap();
    let session = init.headers()["Mcp-Session-Id"].to_str()
                                                  .unwrap()
                                                  .to_owned();

    let list = client.post(format!("{base}/mcp"))
                     .header("Mcp-Session-Id", &session)
                     .json(&json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }))
                     .send()
                     .await
                     .unwrap();
    assert_eq!(list.headers()["Mcp-Session-Id"].to_str().unwrap(),
               session,
               "the session initialize gave stays the session");

    server.stop();
}

/// The version `initialize` answers with, for a client asking for
/// `requested`.
async fn negotiated(base: &str, requested: &str) -> Value {
    let body: Value = support::client().post(format!("{base}/mcp"))
                                            .json(&json!({ "jsonrpc": "2.0", "id": 1,
                                                           "method": "initialize",
                                                           "params": { "protocolVersion": requested,
                                                                       "capabilities": {},
                                                                       "clientInfo": { "name": "probe", "version": "0" } } }))
                                            .send()
                                            .await
                                            .unwrap()
                                            .json()
                                            .await
                                            .unwrap();
    body["result"]["protocolVersion"].clone()
}

/// #442: the issue's own reproduction. A version the server implements is
/// echoed; one it does not is answered with the newest it does (MCP
/// 2025-11-25, Lifecycle, "Version Negotiation").
#[tokio::test]
async fn initialize_echoes_a_supported_version_and_otherwise_offers_the_newest() {
    let (mut server, base) = start_server(Arc::new(EmptyCatalog)).await;

    assert_eq!(negotiated(&base, "2025-06-18").await, "2025-06-18");
    assert_eq!(negotiated(&base, "2025-11-25").await, "2025-11-25");
    assert_eq!(negotiated(&base, "2024-11-05").await, "2025-11-25");
    assert_eq!(negotiated(&base, "2099-01-01").await, "2025-11-25");

    server.stop();
}

/// After `initialize`, a request naming a version the server does not serve
/// is refused with 400 (MCP 2025-11-25, Transports, "Protocol Version
/// Header"). The negotiated version, no header at all, and the 2024-11-05
/// Knot answered with before #442 are all served.
#[tokio::test]
async fn a_session_naming_an_unknown_protocol_version_is_refused() {
    let (mut server, base) = start_server(Arc::new(EmptyCatalog)).await;
    let client = support::client();
    let init = client.post(format!("{base}/mcp"))
                     .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                                    "params": { "protocolVersion": "2025-06-18" } }))
                     .send()
                     .await
                     .unwrap();
    let session = init.headers()["Mcp-Session-Id"].to_str()
                                                  .unwrap()
                                                  .to_owned();
    let list = |version: Option<&'static str>| {
        let mut request =
            client.post(format!("{base}/mcp"))
                  .header("Mcp-Session-Id", &session)
                  .json(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
        if let Some(version) = version {
            request = request.header("MCP-Protocol-Version", version);
        }
        request.send()
    };

    assert_eq!(list(Some("2025-06-18")).await.unwrap().status(), 200);
    assert_eq!(list(None).await.unwrap().status(), 200);
    assert_eq!(list(Some("2024-11-05")).await.unwrap().status(), 200);
    let refused = list(Some("2099-01-01")).await.unwrap();
    assert_eq!(refused.status(), 400);
    let body: Value = refused.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);

    server.stop();
}

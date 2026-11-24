//! `Origin` validation (MCP 2025-11-25, Transports, "Security Warning"):
//! no origin and loopback origins are served, any other is refused, on
//! `/mcp` and on the hook endpoints alike.

use super::*;

/// `initialize` on `/mcp`, sent with `origin` as its `Origin` header, or with
/// none.
async fn initialize_from(base: &str, origin: Option<&str>) -> reqwest::Response {
    let mut request = reqwest::Client::new().post(format!("{base}/mcp"))
                                            .json(&json!({ "jsonrpc": "2.0", "id": 1,
                                                           "method": "initialize" }));
    if let Some(origin) = origin {
        request = request.header("Origin", origin);
    }
    request.send().await.unwrap()
}

/// Streamable HTTP requires the server to validate `Origin` and answer an
/// invalid one with 403 (MCP 2025-11-25, Transports, "Security Warning").
/// The agents Knot launches send none, so that is served; so is a page on
/// the loopback interface.
#[tokio::test]
async fn a_request_with_no_origin_or_a_loopback_origin_is_served() {
    let (mut server, base) = start_server(Arc::new(EmptyCatalog)).await;

    for origin in [None,
                   Some("http://localhost:3000"),
                   Some("http://127.0.0.1"),
                   Some("http://[::1]:8767")]
    {
        assert_eq!(initialize_from(&base, origin).await.status(),
                   200,
                   "{origin:?}");
    }

    server.stop();
}

#[tokio::test]
async fn a_request_from_any_other_origin_is_refused() {
    let (mut server, base) = start_server(Arc::new(EmptyCatalog)).await;

    for origin in ["https://example.com", "null"] {
        let refused = initialize_from(&base, Some(origin)).await;
        assert_eq!(refused.status(), 403, "{origin}");
        let body: Value = refused.json().await.unwrap();
        assert!(body["error"]["code"].is_i64(), "a JSON-RPC error: {body}");
        assert!(body["id"].is_null(), "with no id: {body}");
    }

    server.stop();
}

/// The hook endpoints change agent state as `/mcp` does, so they are held to
/// the same rule.
#[tokio::test]
async fn the_hook_endpoints_refuse_a_foreign_origin() {
    let (mut server, base) = start_server(Arc::new(EmptyCatalog)).await;
    let client = reqwest::Client::new();

    for path in ["/api/v1/agent/status", "/api/v1/agent/register"] {
        let refused = client.post(format!("{base}{path}"))
                            .header("Origin", "https://example.com")
                            .json(&json!({}))
                            .send()
                            .await
                            .unwrap();
        assert_eq!(refused.status(), 403, "{path}");
    }

    server.stop();
}

//! Lists a real HTTP MCP server's tools, named by `KNOT_MCP_LIVE_URL`.
//!
//! Ignored by default: it needs a server running and goes over the network.
//! Run with `KNOT_MCP_LIVE_URL=http://127.0.0.1:8767/mcp cargo test -p
//! knot-mcp-client --test against_a_live_server -- --ignored --nocapture`.

use std::time::Duration;

use knot_mcp_client::{Endpoint, list_tools};

#[test]
#[ignore = "needs a live MCP server named by KNOT_MCP_LIVE_URL"]
fn a_live_server_lists_its_tools() {
    let url = std::env::var("KNOT_MCP_LIVE_URL").expect("KNOT_MCP_LIVE_URL names a server");

    let tools = list_tools(&Endpoint::Http { url }, "", Duration::from_secs(20)).expect("listed");

    println!("{} tools", tools.len());
    for tool in tools.iter().take(5) {
        println!("  {} - {}",
                 tool.name,
                 tool.description
                     .as_deref()
                     .unwrap_or_default()
                     .lines()
                     .next()
                     .unwrap_or_default());
    }
    assert!(!tools.is_empty());
}

//! The conversation itself, independent of how its messages travel:
//! `initialize`, `notifications/initialized`, then `tools/list` page by page.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::consts::{CLIENT_NAME, MAX_PAGES, MAX_TOOLS, PROTOCOL_VERSION};
use crate::error::{ClientError, Result};
use crate::tool::Tool;

/// One way of carrying JSON-RPC messages to a server and back.
pub(crate) trait Exchange {
    /// Sends request `id` and answers the server's response to it, skipping
    /// anything else the server says in between.
    fn request(&mut self, id: u64, message: &Value) -> Result<Value>;

    /// Sends a notification, which has no response.
    fn notify(&mut self, message: &Value) -> Result<()>;
}

#[derive(Deserialize)]
struct ToolsPage {
    #[serde(default)]
    tools:       Vec<Tool>,
    #[serde(rename = "nextCursor", default)]
    next_cursor: Option<String>,
}

/// Lists the server's tools over `exchange`.
pub(crate) fn list_tools_over(exchange: &mut dyn Exchange) -> Result<Vec<Tool>> {
    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": CLIENT_NAME, "version": env!("CARGO_PKG_VERSION") },
        },
    });
    result_of(exchange.request(1, &initialize)?)?;
    exchange.notify(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;

    let mut tools = Vec::new();
    let mut cursor: Option<String> = None;
    for (id, _) in (2_u64..).zip(0..MAX_PAGES) {
        let params = cursor.as_ref()
                           .map_or_else(|| json!({}), |cursor| json!({ "cursor": cursor }));
        let request =
            json!({ "jsonrpc": "2.0", "id": id, "method": "tools/list", "params": params });
        let page: ToolsPage =
            serde_json::from_value(result_of(exchange.request(id, &request)?)?)
                .map_err(|error| ClientError::Protocol(error.to_string()))?;
        tools.extend(page.tools);
        if tools.len() >= MAX_TOOLS {
            tools.truncate(MAX_TOOLS);
            break;
        }
        match page.next_cursor {
            Some(next) if !next.is_empty() => cursor = Some(next),
            _ => break,
        }
    }
    Ok(tools)
}

/// Whether `message` is the response to request `id`, rather than a
/// notification or a request of the server's own.
pub(crate) fn answers(message: &Value, id: u64) -> bool {
    message.get("id").and_then(Value::as_u64) == Some(id)
    && (message.get("result").is_some() || message.get("error").is_some())
}

/// A response's `result`, or its `error` as a [`ClientError::Rpc`].
fn result_of(response: Value) -> Result<Value> {
    if let Some(error) = response.get("error") {
        return Err(ClientError::Rpc { code:    error.get("code")
                                                    .and_then(Value::as_i64)
                                                    .unwrap_or_default(),
                                      message: error.get("message")
                                                    .and_then(Value::as_str)
                                                    .unwrap_or_default()
                                                    .to_owned(), });
    }
    response.get("result")
            .cloned()
            .ok_or_else(|| ClientError::Protocol("a response with no result".to_owned()))
}

#[cfg(test)]
mod tests;

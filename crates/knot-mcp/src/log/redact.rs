//! What a request looks like in the logs: the shape of a tool call and who
//! made it, never the call's content or the connection's credentials.
//!
//! Tool-call arguments carry prompt text, message bodies and file paths, and
//! the logs - the file in the user's Logs directory and standard error alike -
//! end up pasted into public bug reports. So a call is recorded by its
//! *shape*: which tool, which argument names, how much payload. Standard error
//! used to print the arguments in full; it now gets the same description the
//! file does.
//!
//! The caller is named by the agent the connection is bound to, never by its
//! MCP session id: the id is what a client sends back as `Mcp-Session-Id` to
//! act as that connection, so it is a credential, and a log is no place for
//! one (CodeQL `rust/cleartext-logging`).

use serde_json::Value;

use crate::tools::Caller;

/// The log file's description of a `tools/call`: the tool's name, the names
/// of its top-level arguments, and the size of the argument payload.
///
/// Argument *values* never appear. A non-object payload (a bare array or
/// string, which the protocol does not use but a client may still send) has
/// no keys to name, and is reported by size alone.
pub fn describe_call(name: &str, arguments: &Value) -> String {
    let bytes = arguments.to_string().len();
    match arguments.as_object() {
        Some(fields) => {
            let keys: Vec<&str> = fields.keys().map(String::as_str).collect();
            format!("tools/call {name} keys=[{}] bytes={bytes}", keys.join(", "))
        }
        None => format!("tools/call {name} keys=<not an object> bytes={bytes}"),
    }
}

/// Who made a request, for a log line: the agent the connection is bound
/// to, or `unbound` before it is bound to one. The MCP session id is
/// deliberately left out - see the module documentation.
pub fn describe_caller(caller: &Caller) -> String {
    match caller.agent {
        Some(agent) => format!("agent {agent}"),
        None => "unbound".to_string(),
    }
}

#[cfg(test)]
mod tests;

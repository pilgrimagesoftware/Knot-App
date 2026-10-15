//! Streamable HTTP: each message is a `POST`, answered either as one JSON
//! body or as a server-sent event stream carrying it.
//!
//! No credential is sent. A server that wants one says 401 or 403, which is
//! reported as [`ClientError::Unauthorized`]: the sign-in belongs to the
//! agent, and the section never takes it over.

use std::io::{BufRead, BufReader};
use std::time::Instant;

use serde_json::Value;
use ureq::Agent;

use crate::consts::PROTOCOL_VERSION;
use crate::error::{ClientError, Result};
use crate::session::{Exchange, answers};

/// Response bodies are read at most this far. A `tools/list` page is a few
/// kilobytes; this is a ceiling against a misbehaving server, not a budget.
const MAX_BODY_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) struct HttpExchange {
    url:      String,
    deadline: Instant,
    /// `Mcp-Session-Id`, once `initialize` has issued one.
    session:  Option<String>,
}

impl HttpExchange {
    pub(crate) fn new(url: &str, deadline: Instant) -> Self {
        Self { url: url.to_owned(),
               deadline,
               session: None }
    }

    /// The time left, or [`ClientError::TimedOut`] once there is none.
    fn remaining(&self) -> Result<std::time::Duration> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(ClientError::TimedOut);
        }
        Ok(left)
    }

    fn post(&mut self, message: &Value) -> Result<ureq::http::Response<ureq::Body>> {
        let agent: Agent = Agent::config_builder().timeout_global(Some(self.remaining()?))
                                                  .http_status_as_error(false)
                                                  .build()
                                                  .into();
        let mut request = agent.post(&self.url)
                               .header("Content-Type", "application/json")
                               .header("Accept", "application/json, text/event-stream");
        if let Some(session) = &self.session {
            request = request.header("Mcp-Session-Id", session)
                             .header("MCP-Protocol-Version", PROTOCOL_VERSION);
        }
        let response =
            request.send(message.to_string())
                   .map_err(|error| match error {
                       ureq::Error::Timeout(_) => ClientError::TimedOut,
                       other => ClientError::Transport(other.to_string()),
                   })?;
        match response.status().as_u16() {
            401 | 403 => Err(ClientError::Unauthorized(response.status().as_u16())),
            200..=299 => {
                if let Some(session) = response.headers()
                                               .get("mcp-session-id")
                                               .and_then(|value| value.to_str().ok())
                {
                    self.session = Some(session.to_owned());
                }
                Ok(response)
            }
            status => Err(ClientError::Http(status)),
        }
    }
}

impl Exchange for HttpExchange {
    fn request(&mut self, id: u64, message: &Value) -> Result<Value> {
        let mut response = self.post(message)?;
        let is_stream = response.headers()
                                .get("content-type")
                                .and_then(|value| value.to_str().ok())
                                .is_some_and(|kind| kind.starts_with("text/event-stream"));
        let reader = response.body_mut()
                             .with_config()
                             .limit(MAX_BODY_BYTES)
                             .reader();
        if is_stream {
            return answer_in_stream(BufReader::new(reader), id);
        }
        let body: Value =
            serde_json::from_reader(reader).map_err(|error| {
                                               ClientError::Protocol(error.to_string())
                                           })?;
        answer_in_body(body, id)
    }

    fn notify(&mut self, message: &Value) -> Result<()> {
        // 202 with no body is the expected answer; the body is not read.
        self.post(message).map(drop)
    }
}

/// The response to `id` in a plain JSON body, which may be one message or a
/// batch.
fn answer_in_body(body: Value, id: u64) -> Result<Value> {
    let found = match body {
        Value::Array(batch) => batch.into_iter().find(|message| answers(message, id)),
        single => answers(&single, id).then_some(single),
    };
    found.ok_or_else(|| ClientError::Protocol(format!("no response to request {id}")))
}

/// The response to `id` in a server-sent event stream, read event by event
/// and stopped as soon as it arrives: the server may keep the stream open
/// after answering, so reading to the end would wait for the timeout.
pub(crate) fn answer_in_stream(reader: impl BufRead, id: u64) -> Result<Value> {
    let mut data = String::new();
    for line in reader.lines() {
        let line = line?;
        if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.strip_prefix(' ').unwrap_or(value));
            continue;
        }
        if line.is_empty() && !data.is_empty() {
            if let Ok(message) = serde_json::from_str::<Value>(&data)
               && answers(&message, id)
            {
                return Ok(message);
            }
            data.clear();
        }
    }
    // A stream ending without the blank line that closes its last event.
    serde_json::from_str::<Value>(&data).ok()
                                        .filter(|message| answers(message, id))
                                        .ok_or(ClientError::Closed)
}

#[cfg(test)]
mod tests;

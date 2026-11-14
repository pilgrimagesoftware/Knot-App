//! `agy mcp list`.
//!
//! A padded table. Like Codex's listing it is configuration, not a probe:
//! STATUS is `enabled` or `disabled`, and `agy` printed `enabled` for a
//! server whose URL nothing was listening on (checked against agy 1.5.8). An
//! enabled row is therefore [`ServerState::Unknown`], never `Connected`.
//!
//! ```text
//! NAME   TYPE   STATUS    COMMAND/URL
//! local  stdio  disabled  /usr/bin/env FOO bar
//! notes  http   enabled   http://127.0.0.1:9/mcp
//! ```
//!
//! With nothing configured it prints `No MCP servers configured.` instead.
//! A stdio row's command is one string, so no launch is recovered from it.

use crate::server::{ServerRow, Target};
use crate::state::ServerState;

const HEADER_START: &str = "NAME";
const EMPTY_MARKER: &str = "no mcp servers configured";

/// Reads a listing, or `None` when it is not this table.
pub(crate) fn parse(output: &str) -> Option<Vec<ServerRow>> {
    let mut lines = output.lines()
                          .map(str::trim)
                          .filter(|line| !line.is_empty());
    let first = lines.next()?;

    if first.to_lowercase().starts_with(EMPTY_MARKER) {
        return Some(Vec::new());
    }
    if !(first.starts_with(HEADER_START) && first.contains("STATUS")) {
        return None;
    }

    lines.map(row_from).collect()
}

/// One row: three words, then the rest of the line as the target.
fn row_from(line: &str) -> Option<ServerRow> {
    let (name, rest) = split_word(line)?;
    let (transport, rest) = split_word(rest)?;
    let (status, target) = split_word(rest)?;

    let target = match transport {
        "http" | "sse" => Target::Http { url: target.to_owned(), },
        "stdio" => Target::Stdio { command: target.to_owned(), },
        _ => return None,
    };
    let state = match status {
        "enabled" => ServerState::Unknown,
        "disabled" => ServerState::Disabled,
        _ => return None,
    };

    Some(ServerRow::new(name.to_owned(), target, state))
}

/// The first word and the rest of the line, without its leading blanks.
fn split_word(text: &str) -> Option<(&str, &str)> {
    let (word, rest) = text.split_once(char::is_whitespace)?;
    let rest = rest.trim_start();

    (!rest.is_empty()).then_some((word, rest))
}

#[cfg(test)]
mod tests;

//! What a log loses before a report publishes it (#284).
//!
//! An attached log goes into a public GitHub issue. Knot's own lines no
//! longer print tool-call arguments or session ids (PR #274), but a log on
//! disk can predate that, and much of the application log is not Knot's at
//! all: ACP adapters and MCP servers write their own stderr into it. So the
//! scrub runs on the text being attached, whoever wrote it.
//!
//! It removes, line by line:
//!
//! - a `tools/call`'s argument values, keeping its name, argument keys and size
//!   - [`knot_mcp::describe_call`], the shape the MCP log file records;
//! - MCP and ACP session ids, `Mcp-Session-Id` values included;
//! - credentials: auth headers, `Bearer` tokens, `key=`/`token=`-style values,
//!   GitHub, Anthropic, OpenAI, Slack and AWS key formats, and passwords in
//!   URLs;
//! - email addresses;
//! - the user's home directory, which becomes `~`.
//!
//! The scrub errs towards removing too much. A report reader can ask for a
//! detail; nobody can take a published token back.

use std::sync::LazyLock;

use regex::{Captures, Regex};

/// What a removed value is replaced with.
const REDACTED: &str = "<redacted>";

/// One scrubbing rule: a pattern and what each match becomes.
struct Rule {
    pattern:     Regex,
    replacement: &'static str,
}

impl Rule {
    fn new(pattern: &str, replacement: &'static str) -> Self {
        Self { pattern: Regex::new(pattern).expect("scrub patterns are fixed and valid"),
               replacement }
    }
}

/// `... tools/call <name> <arguments as JSON>`: the stderr line that printed
/// a call's arguments in full before PR #274.
static TOOL_CALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<lead>.*?)\btools/call (?P<name>[\w.:-]+) (?P<args>[\[{].*)$")
        .expect("the tool-call pattern is fixed and valid")
});

/// An email address, kept apart from [`RULES`] so `git@` remote URLs - a
/// host, not a person - survive.
static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?P<local>[A-Za-z0-9._%+-]+)@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b")
        .expect("the email pattern is fixed and valid")
});

/// Everything else, in order. Credentials embedded in a larger value go
/// first, so a later, broader rule cannot leave half of one behind.
static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![// A password in a URL: `https://user:secret@host`.
         Rule::new(r"(?P<scheme>://)[^/\s:@]+:[^/\s@]+@",
                   "${scheme}<redacted>@"),
         // Known token formats, wherever they appear.
         Rule::new(r"\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})",
                   "<github-token>"),
         Rule::new(r"\bsk-(?:ant-|proj-)?[A-Za-z0-9_-]{20,}", "<api-key>"),
         Rule::new(r"\bxox[abprs]-[A-Za-z0-9-]{10,}", "<api-key>"),
         Rule::new(r"\bA(?:KIA|SIA)[0-9A-Z]{16}\b", "<api-key>"),
         // A named secret: the name stays, its value goes. Covers headers
         // (`Authorization: Bearer ...`), query strings and JSON fields.
         Rule::new(r#"(?i)(?P<name>\b(?:proxy-)?authorization|\bx-api-key|\bapi[_-]?key|\baccess[_-]?token|\brefresh[_-]?token|\bauth[_-]?token|\bid[_-]?token|\bclient[_-]?secret|\bsecret|\bpassword|\bpasswd|\btoken)(?P<sep>"?\s*[:=]\s*"?)(?:(?:bearer|basic|token)\s+)?[^\s"',;}&]+"#,
                   "${name}${sep}<redacted>"),
         // A bearer token outside any named header.
         Rule::new(r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{8,}",
                   "Bearer <redacted>"),
         // Session ids: `Mcp-Session-Id: ...`, `sessionId=...`,
         // `"session_id":"..."`.
         Rule::new(r#"(?i)(?P<name>\bmcp-session-id|\bsession_?id)(?P<sep>"?\s*[:=]\s*"?)[^\s"',;}&]+"#,
                   "${name}${sep}<redacted>"),
         // `knot-mcp: [<session uuid>] -> method`: the request and response
         // lines named the connection by its MCP session before PR #274.
         Rule::new(r"\[[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\]",
                   "[<session>]"),]
});

/// `log` with everything [the module](self) lists removed. `home` is the
/// user's home directory, replaced with `~` wherever it appears.
pub(super) fn scrub(log: &str, home: Option<&str>) -> String {
    let home = home.and_then(home_pattern);
    log.split('\n')
       .map(|line| scrub_line(line, home.as_ref()))
       .collect::<Vec<_>>()
       .join("\n")
}

fn scrub_line(line: &str, home: Option<&Regex>) -> String {
    let mut line = without_tool_arguments(line);
    for rule in RULES.iter() {
        line = rule.pattern
                   .replace_all(&line, rule.replacement)
                   .into_owned();
    }
    line = EMAIL.replace_all(&line, |captures: &Captures<'_>| {
                    if &captures["local"] == "git" {
                        captures[0].to_owned()
                    }
                    else {
                        "<email>".to_owned()
                    }
                })
                .into_owned();
    match home {
        Some(home) => home.replace_all(&line, "~${end}").into_owned(),
        None => line,
    }
}

/// A `tools/call` line with its arguments reduced to their shape. Arguments
/// that do not parse - a line cut short by the tail, say - go entirely.
fn without_tool_arguments(line: &str) -> String {
    let Some(captures) = TOOL_CALL.captures(line)
    else {
        return line.to_owned();
    };
    let lead = &captures["lead"];
    let name = &captures["name"];
    match serde_json::from_str::<serde_json::Value>(&captures["args"]) {
        Ok(arguments) => format!("{lead}{}", knot_mcp::describe_call(name, &arguments)),
        Err(_) => format!("{lead}tools/call {name} {REDACTED}"),
    }
}

/// `text` with the home directory `home` written as `~`. Only a whole path
/// component matches, so `/Users/ann` leaves `/Users/anna` alone.
pub(super) fn home_relative(text: &str, home: &str) -> String {
    match home_pattern(home) {
        Some(pattern) => pattern.replace_all(text, "~${end}").into_owned(),
        None => text.to_owned(),
    }
}

/// Matches `home` as a whole leading run of path components, capturing what
/// follows it as `end`. `None` for an empty home, which would match
/// everywhere.
fn home_pattern(home: &str) -> Option<Regex> {
    let home = home.trim_end_matches('/');
    if home.is_empty() {
        return None;
    }
    Regex::new(&format!(r"{}(?P<end>/|\b|$)", regex::escape(home))).ok()
}

/// The user's home directory, as the scrub needs it.
pub(super) fn home_dir() -> Option<String> {
    std::env::var("HOME").ok().filter(|home| !home.is_empty())
}

#[cfg(test)]
mod tests;

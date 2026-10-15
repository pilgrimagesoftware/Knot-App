//! The scrub against log lines as they are actually written - the
//! `knot-mcp`/`knot-acp` lines are copied from issue #267's attached log -
//! plus one line per kind of secret.

use super::{home_relative, scrub};

const HOME: &str = "/Users/someone";

fn scrubbed(line: &str) -> String {
    scrub(line, Some(HOME))
}

/// The line #284 is about: a `send-message` printed with its whole body.
#[test]
fn a_tool_calls_arguments_are_reduced_to_their_shape() {
    let line = r##"knot-mcp: tools/call send-message {"from":"96ef404f-b11e-4a39-af51-8c53ae29a001","to":"11b276cb-2b50-45ba-84ca-7ddc26c770e3","content":"#233 gpui-kit migration is done. Commit 33ac063 on feature/233-migrate-to-gpui-kit."}"##;

    let out = scrubbed(line);

    assert!(out.starts_with("knot-mcp: tools/call send-message keys=[from, to, content] bytes="),
            "{out}");
    assert!(!out.contains("migration is done"),
            "the message body must go: {out}");
    assert!(!out.contains("96ef404f"), "argument values must go: {out}");
}

#[test]
fn a_tool_call_cut_short_loses_its_arguments_entirely() {
    let out = scrubbed(r#"knot-mcp: tools/call send-message {"content":"half a mess"#);

    assert_eq!(out, "knot-mcp: tools/call send-message <redacted>");
}

/// The request and response lines named the connection by its MCP session.
#[test]
fn an_mcp_session_in_brackets_is_redacted() {
    for line in ["knot-mcp: [68b52193-c5f3-4788-93e7-780424198adf] <- tools/call ok",
                 "knot-mcp: [0301232b-d9d3-415b-926a-f51f4804883a] -> tools/call"]
    {
        let out = scrubbed(line);
        assert!(out.starts_with("knot-mcp: [<session>] "), "{out}");
    }
}

#[test]
fn session_ids_in_headers_fields_and_parameters_are_redacted() {
    let cases = [("Mcp-Session-Id: 68b52193-c5f3-4788-93e7-780424198adf", "68b52193"),
                 ("[session/create] sessionId=4b78b512-d3f0-4303-85e3-8e53ebe8b28f phase=settings",
                  "4b78b512"),
                 (r#"Unexpected case: {"type":"system","session_id":"0a8f01c7-317b-48a3-af82-e2c1648522ef"}"#,
                  "0a8f01c7")];
    for (line, id) in cases {
        let out = scrubbed(line);
        assert!(!out.contains(id), "{out}");
        assert!(out.contains("<redacted>"), "{out}");
    }
}

#[test]
fn credentials_are_redacted_and_their_names_kept() {
    let cases = ["Authorization: Bearer abcdefghijklmnopqrstuvwxyz0123",
                 "x-api-key: sk-ant-api03-abcdefghijklmnopqrstuvwxyz",
                 r#"{"access_token":"eyJhbGciOiJIUzI1NiJ9.payload.signature"}"#,
                 "curl https://api.example.com/v1?api_key=abcdef0123456789",
                 "password=hunter2hunter2"];
    for line in cases {
        let out = scrubbed(line);
        assert!(out.contains("<redacted>"), "{line} -> {out}");
        for secret in ["abcdefghijklmnopqrstuvwxyz",
                       "eyJhbGci",
                       "abcdef0123456789",
                       "hunter2"]
        {
            assert!(!out.contains(secret), "{line} -> {out}");
        }
    }
    assert!(scrubbed("Authorization: Bearer abcdefghijklmnop").starts_with("Authorization: "),
            "the header's name stays, so the reader knows what was there");
}

#[test]
fn known_token_formats_are_redacted_anywhere() {
    let cases = [("pushing with ghp_0123456789abcdefghijABCDEFGHIJ012345", "<github-token>"),
                 ("token github_pat_11ABCDEFG0123456789_abcdefghijklmnopqrstuvwxyz",
                  "<github-token>"),
                 ("gho_0123456789abcdefghijABCDEFGHIJ in the env", "<github-token>"),
                 ("key sk-proj-abcdefghijklmnopqrstuvwx", "<api-key>"),
                 ("AKIAIOSFODNN7EXAMPLE", "<api-key>")];
    for (line, marker) in cases {
        let out = scrubbed(line);
        assert!(out.contains(marker), "{line} -> {out}");
    }
}

#[test]
fn a_password_in_a_url_is_redacted() {
    let out = scrubbed("cloning https://someone:s3cretpass@example.com/repo.git");

    assert_eq!(out, "cloning https://<redacted>@example.com/repo.git");
}

#[test]
fn email_addresses_are_redacted_but_a_git_remote_is_not() {
    assert_eq!(scrubbed("author pschifferer@example.com committed"),
               "author <email> committed");
    assert_eq!(scrubbed("remote git@github.com:pilgrimagesoftware/Knot-App.git"),
               "remote git@github.com:pilgrimagesoftware/Knot-App.git");
}

#[test]
fn the_home_directory_becomes_a_tilde() {
    let out = scrubbed("cwd=/Users/someone/Projects/Code/Libri/dtrpg");

    assert_eq!(out, "cwd=~/Projects/Code/Libri/dtrpg");
}

#[test]
fn only_a_whole_home_directory_matches() {
    assert_eq!(home_relative("/Users/someoneelse/file", HOME),
               "/Users/someoneelse/file");
    assert_eq!(home_relative("/Users/someone", HOME), "~");
    assert_eq!(home_relative("/Users/someone/", HOME), "~/");
}

/// Most of a log is nothing secret, and a report is only useful if that
/// part survives untouched.
#[test]
fn ordinary_lines_are_left_alone() {
    for line in ["knot-acp: [claude-agent-acp] -> session/prompt (id 25)",
                 "knot-acp: [claude-agent-acp] <- session/prompt (id 21) ok",
                 "knot-mcp: tools/list -> 18 tools: register-agent, list-agents, send-message",
                 "knot-mcp: tools/call set-status keys=[agentId, status] bytes=88 from agent 1"]
    {
        assert_eq!(scrubbed(line), line);
    }
}

#[test]
fn every_line_of_a_log_is_scrubbed_and_the_line_count_kept() {
    let log =
        "knot-mcp: [68b52193-c5f3-4788-93e7-780424198adf] -> tools/call\nmail me at a@b.io\nplain";

    let out = scrub(log, None);

    assert_eq!(out.lines().count(), 3);
    assert_eq!(out,
               "knot-mcp: [<session>] -> tools/call\nmail me at <email>\nplain");
}

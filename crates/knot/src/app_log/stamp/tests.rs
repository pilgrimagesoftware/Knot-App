use std::io::Write;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::*;

/// Runs `writes` through a real pipe into the stamping loop and returns
/// what reached the log, with how many flush markers were passed.
fn through_pipe(writes: &[&[u8]]) -> (String, u64) {
    let (reader, mut writer) = std::io::pipe().expect("pipe");
    let drained = Drained::default();
    let mut log = Vec::new();
    std::thread::scope(|scope| {
        let copier = scope.spawn(|| copy_stamped(reader, &mut log, &drained));
        for bytes in writes {
            writer.write_all(bytes).expect("write");
        }
        drop(writer);
        copier.join().expect("copier");
    });
    (String::from_utf8(log).expect("utf-8"), drained.seen())
}

/// Splits one log line into its time and the text logged.
fn parts(line: &str) -> (OffsetDateTime, &str) {
    let (time, text) = line.split_once(' ').expect("a time, then the line");
    (OffsetDateTime::parse(time, &Rfc3339).expect("an RFC 3339 time"), text)
}

#[test]
fn every_line_is_written_behind_a_utc_time() {
    let (log, _) = through_pipe(&[b"first\nsecond\n"]);

    let lines: Vec<_> = log.lines().map(parts).collect();
    assert_eq!(lines.iter().map(|(_, text)| *text).collect::<Vec<_>>(),
               ["first", "second"]);
    for (time, _) in lines {
        assert!(time.offset().is_utc());
    }
}

#[test]
fn a_line_written_in_pieces_is_one_line() {
    let (log, _) = through_pipe(&[b"[claude-agent-acp 42] half", b" and the rest\n"]);

    let texts: Vec<_> = log.lines().map(|line| parts(line).1).collect();
    assert_eq!(texts, ["[claude-agent-acp 42] half and the rest"]);
}

#[test]
fn an_empty_line_is_kept() {
    let (log, _) = through_pipe(&[b"above\n\nbelow\n"]);

    let texts: Vec<_> = log.lines().map(|line| parts(line).1).collect();
    assert_eq!(texts, ["above", "", "below"]);
}

#[test]
fn a_last_line_without_a_newline_is_still_logged() {
    let (log, _) = through_pipe(&[b"done\nthread 'main' panicked"]);

    let texts: Vec<_> = log.lines().map(|line| parts(line).1).collect();
    assert_eq!(texts, ["done", "thread 'main' panicked"]);
}

#[test]
fn a_flush_marker_is_counted_and_not_logged() {
    let mut marker = FLUSH_MARKER.to_vec();
    marker.push(b'\n');

    let (log, flushes) = through_pipe(&[b"before\n", &marker, b"after\n"]);

    let texts: Vec<_> = log.lines().map(|line| parts(line).1).collect();
    assert_eq!(texts, ["before", "after"]);
    assert_eq!(flushes, 1);
}

#[test]
fn an_unfinished_line_ahead_of_a_flush_marker_is_still_logged() {
    let mut marker = FLUSH_MARKER.to_vec();
    marker.push(b'\n');

    let (log, flushes) = through_pipe(&[b"partial", &marker]);

    let texts: Vec<_> = log.lines().map(|line| parts(line).1).collect();
    assert_eq!(texts, ["partial"]);
    assert_eq!(flushes, 1);
}

#[test]
fn a_flush_waits_for_the_marker_and_gives_up_after_its_timeout() {
    let drained = Drained::default();
    assert!(!drained.wait_past(0, std::time::Duration::from_millis(10)));

    std::thread::scope(|scope| {
        scope.spawn(|| drained.mark());
        assert!(drained.wait_past(0, std::time::Duration::from_secs(5)));
    });
}

/// Security review of #521: an adapter's stderr line that carries an MCP
/// credential reaches the log file without it.
#[test]
fn a_credential_in_a_logged_line_is_replaced() {
    for kind in [knot_mcp::CredentialKind::AgentTool,
                 knot_mcp::CredentialKind::AgentHooks,
                 knot_mcp::CredentialKind::Client]
    {
        let secret = knot_mcp::Secret::generate(kind);
        let line = format!("[claude-agent-acp 42] invalid mcp config: {{\"headers\":[{{\"value\":\"Bearer {}\"}}]}}",
                           secret.expose());
        let out = String::from_utf8(super::stamped(time::OffsetDateTime::UNIX_EPOCH,
                                                   line.as_bytes())).unwrap();
        assert!(!out.contains(secret.expose()), "{out}");
        assert!(out.contains("<knot-credential>"), "{out}");
    }
    let plain = super::redact_credentials(b"knot-acp: [claude 1] -> session/new (id 2)");
    assert!(matches!(plain, std::borrow::Cow::Borrowed(_)),
            "an ordinary line is not copied");
}

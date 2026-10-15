//! The browser fallback names scrubbed copies, never the logs themselves
//! (#284).

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use knot_forge::ForgeAvailability;
use tempfile::TempDir;

use super::{deliver, write};
use crate::bug_report::kind::IssueKind;
use crate::bug_report::logs::{Attachment, LogKind};
use crate::bug_report::submit::{Outcome, Report, compose_body};

/// Where a log lives on disk, which the fallback must never name.
const RAW: &str = "/Users/someone/Library/Logs/Knot/knot.log";

fn attachment(kind: LogKind, tail: Option<&str>) -> Attachment {
    Attachment { kind,
                 path: Some(PathBuf::from(RAW)),
                 tail: tail.map(str::to_owned) }
}

fn report(attachments: Vec<Attachment>) -> Report {
    Report { kind: IssueKind::Bug,
             subject: "Something broke".to_string(),
             description: "It broke.".to_string(),
             diagnostics: "App: Knot".to_string(),
             attachments,
             screenshots: Vec::new() }
}

/// Every file under `dir`, at any depth.
fn copies(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir)
    else {
        return Vec::new();
    };
    entries.flatten()
           .flat_map(|entry| {
               let path = entry.path();
               if path.is_dir() {
                   copies(&path)
               }
               else {
                   vec![path]
               }
           })
           .collect()
}

#[test]
fn each_attachment_is_pointed_at_a_copy_of_its_scrubbed_tail() {
    let root = TempDir::new().expect("a temporary directory");
    let attachments = [attachment(LogKind::App, Some("scrubbed app tail")),
                       attachment(LogKind::Mcp, Some("scrubbed mcp tail"))];

    let copied = write(&attachments, root.path());

    for (copy, tail) in copied.iter()
                              .zip(["scrubbed app tail", "scrubbed mcp tail"])
    {
        let path = copy.path.as_deref().expect("a copy was written");
        assert!(path.starts_with(root.path()),
                "{path:?} is not under the copies folder");
        assert_eq!(std::fs::read_to_string(path).expect("the copy reads back"),
                   tail);
        assert_eq!(copy.tail.as_deref(),
                   Some(tail),
                   "the tail itself is unchanged");
    }
}

/// With no scrubbed text to copy, the attachment must not fall back to
/// naming the log itself.
#[test]
fn an_unreadable_log_names_no_file_at_all() {
    let root = TempDir::new().expect("a temporary directory");

    let copied = write(&[attachment(LogKind::App, None)], root.path());

    assert_eq!(copied[0].path, None);
    assert!(copies(root.path()).is_empty());
}

#[test]
fn a_new_report_clears_the_last_ones_copies() {
    let root = TempDir::new().expect("a temporary directory");
    let attachments = [attachment(LogKind::App, Some("tail"))];

    write(&attachments, root.path());
    write(&attachments, root.path());

    assert_eq!(copies(root.path()).len(),
               1,
               "only the latest report's copy remains");
}

/// The reported gap: the compose page told the user to drag in the raw log.
#[test]
fn the_browser_fallback_names_and_shows_the_copies_not_the_logs() {
    let root = TempDir::new().expect("a temporary directory");
    let sent = RefCell::new(None);
    let revealed = RefCell::new(Vec::new());

    let outcome = deliver(&report(vec![attachment(LogKind::App, Some("tail"))]),
                          &ForgeAvailability::Missing,
                          root.path(),
                          |report| {
                              *sent.borrow_mut() = Some(compose_body(report));
                              Outcome::BrowserReady
                          },
                          |path| {
                              revealed.borrow_mut().push(path.to_path_buf());
                              true
                          });

    assert_eq!(outcome, Outcome::BrowserReady);
    let body = sent.into_inner().expect("the report reached submit");
    assert!(!body.contains(RAW),
            "the compose page named the raw log: {body}");
    let revealed = revealed.into_inner();
    assert_eq!(revealed.len(), 1);
    assert!(revealed[0].starts_with(root.path()));
    let copy_name = revealed[0].file_name()
                               .expect("a copy is a file")
                               .to_string_lossy()
                               .into_owned();
    assert!(body.contains(&copy_name),
            "the page names the copy Finder shows: {body}");
}

#[test]
fn a_browser_that_did_not_open_shows_nothing() {
    let root = TempDir::new().expect("a temporary directory");
    let revealed = RefCell::new(0);

    deliver(&report(vec![attachment(LogKind::App, Some("tail"))]),
            &ForgeAvailability::Missing,
            root.path(),
            |_| Outcome::BrowserFailed,
            |_| {
                *revealed.borrow_mut() += 1;
                true
            });

    assert_eq!(revealed.into_inner(), 0);
}

/// A filed report carries its logs in the body; nothing is written.
#[test]
fn a_filed_report_writes_no_copies() {
    let root = TempDir::new().expect("a temporary directory");
    let original = report(vec![attachment(LogKind::App, Some("tail"))]);

    deliver(&original,
            &ForgeAvailability::Ready,
            root.path(),
            |report| {
                assert_eq!(report, &original, "a filed report goes as it is");
                Outcome::Filed("https://github.com/o/r/issues/1".to_string())
            },
            |_| true);

    assert!(copies(root.path()).is_empty());
}

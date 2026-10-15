//! Scrubbed copies of the attached logs, for the browser fallback (#284).
//!
//! A filed report carries its logs in the issue body. The browser fallback
//! cannot - a compose URL that long is refused - so it names files for the
//! user to drag onto the page. Naming the logs themselves would publish them
//! unscrubbed, so the fallback names copies instead: each attachment's tail,
//! already scrubbed by [`super::logs::read`], written to a folder of its own
//! under [`BUG_REPORT_LOG_COPIES_DIR`] in the temporary directory.
//!
//! The copies outlive the report window on purpose: the user drags them from
//! Finder after the window has done its part. They are cleared at the next
//! launch, and before the next report writes its own.

use std::path::{Path, PathBuf};

use knot_forge::ForgeAvailability;

use super::logs::{Attachment, LogKind};
use super::submit::{Outcome, Report};
use crate::consts::BUG_REPORT_LOG_COPIES_DIR;

/// Where the copies live.
pub(super) fn root() -> PathBuf {
    std::env::temp_dir().join(BUG_REPORT_LOG_COPIES_DIR)
}

/// Removes every copy under `root`. A folder already gone is not an error;
/// nothing else is reported either, since there is nobody to tell at launch
/// and a stale copy is already scrubbed.
pub(super) fn clear(root: &Path) {
    let _ = std::fs::remove_dir_all(root);
}

/// Hands `report` to `submit`, the way the window files it.
///
/// When `forge` is ready the report is filed as it is, its logs inside the
/// issue body. Otherwise its attachments are pointed at scrubbed copies under
/// `root` before `submit` sees them, so the compose page names the copies,
/// and once the page is open `reveal` shows each copy in Finder.
pub(super) fn deliver(report: &Report, forge: &ForgeAvailability, root: &Path,
                      submit: impl FnOnce(&Report) -> Outcome, reveal: impl Fn(&Path) -> bool)
                      -> Outcome {
    if forge.is_ready() || report.attachments.is_empty() {
        return submit(report);
    }
    let report = Report { attachments: write(&report.attachments, root),
                          ..report.clone() };
    let outcome = submit(&report);
    if outcome == Outcome::BrowserReady {
        for path in report.attachments
                          .iter()
                          .filter_map(|attachment| attachment.path.as_deref())
        {
            reveal(path);
        }
    }
    outcome
}

/// Writes each attachment's scrubbed tail to a fresh folder under `root`,
/// first clearing the previous report's, and returns the attachments pointed
/// at the copies.
///
/// An attachment with nothing to copy - its log could not be read - or whose
/// copy could not be written loses its path rather than keep the log's own:
/// the fallback must never name the unscrubbed file.
pub(super) fn write(attachments: &[Attachment], root: &Path) -> Vec<Attachment> {
    clear(root);
    let folder = root.join(uuid::Uuid::new_v4().to_string());
    let created = std::fs::create_dir_all(&folder).is_ok();
    attachments.iter()
               .map(|attachment| {
                   let copy = attachment.tail
                                        .as_deref()
                                        .filter(|_| created)
                                        .and_then(|tail| {
                                            let path = folder.join(file_name(attachment.kind));
                                            std::fs::write(&path, tail).ok().map(|()| path)
                                        });
                   Attachment { path: copy,
                                ..attachment.clone() }
               })
               .collect()
}

/// A copy's file name: which log it is, readable on the issue it lands on.
fn file_name(kind: LogKind) -> &'static str {
    match kind {
        LogKind::App => "knot-application.log",
        LogKind::Mcp => "knot-mcp-server.log",
    }
}

#[cfg(test)]
mod tests;

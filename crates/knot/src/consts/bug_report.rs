//! The bug report window's content limits and the application log files.

/// The bug report's description, in rows. Fixed rather than growing with its
/// content: the dialog scrolls its body once it outgrows the window, so a
/// description that grew with a long paste pushed the diagnostics out of
/// sight under it (#515). The field scrolls instead.
pub(crate) const BUG_REPORT_DESCRIPTION_ROWS: usize = 8;

/// How much of each log a bug report attaches: its last this-many lines...
pub(crate) const BUG_REPORT_LOG_TAIL_LINES: usize = 200;

/// ...and at most this many bytes of them. GitHub caps an issue body at
/// 65,536 characters, and two logs have to fit beside the description.
pub(crate) const BUG_REPORT_LOG_TAIL_BYTES: usize = 24 * 1024;

/// The folder under the system temporary directory that holds the scrubbed
/// log copies a browser-fallback report names for the user to drag in
/// (#284). Cleared at launch and before each new set is written.
pub(crate) const BUG_REPORT_LOG_COPIES_DIR: &str = "knot-report-logs";

/// How many screenshots a report can name (#566). The ask is "a couple"; the
/// cap keeps an accidental select-all in a screenshots folder from revealing
/// dozens of files in Finder.
pub(crate) const BUG_REPORT_MAX_SCREENSHOTS: usize = 5;

/// The image types a report keeps as screenshots, lowercased: the ones
/// GitHub renders inline in an issue. The file picker cannot filter, so
/// everything else is skipped after the choice.
pub(crate) const BUG_REPORT_SCREENSHOT_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif"];

/// The application log, in `knot_core::log_dir()`: whatever the process
/// wrote to stderr, when that was not a terminal (`app_log`).
pub(crate) const APP_LOG_FILE_NAME: &str = "knot.log";

/// The previous run's application log, kept beside the current one.
pub(crate) const APP_LOG_PREVIOUS_FILE_NAME: &str = "knot.log.1";

/// How long a panic or an exit waits for stderr's last lines to reach the
/// application log. The stamping thread takes microseconds; this bounds a
/// stuck one, which must not hold up a quit.
pub(crate) const APP_LOG_FLUSH_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

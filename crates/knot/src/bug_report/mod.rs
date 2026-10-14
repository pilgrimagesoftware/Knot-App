//! Help > Report an Issue: the window that collects a report and files it as
//! a GitHub issue (`openspec/specs/bug-reporting`).
//!
//! Its own window, like About, rather than a dialog on the active window
//! (#39): the report can sit beside what it describes, and opens even with
//! no other window.
//!
//! `form` decides when Report is available and what the status line says,
//! `diagnostics` gathers and lays out what accompanies every report, `logs`
//! reads the logs a user chooses to attach,
//! `submit` turns a report into a filed issue or a pre-filled browser page,
//! `report` holds what was typed and runs the blocking work, `pane` draws
//! the form, and `window` opens the window and hosts the two.

mod diagnostics;
mod form;
mod kind;
mod logs;
mod pane;
mod report;
mod screenshots;
mod submit;
mod window;

#[cfg(test)]
pub(crate) use diagnostics::Diagnostics;
#[cfg(test)]
pub(crate) use kind::IssueKind;
#[cfg(test)]
pub(crate) use logs::{Attachment, LogKind};
#[cfg(test)]
pub(crate) use report::{BugReport, ReportServices};
#[cfg(test)]
pub(crate) use submit::{Outcome, Report};
pub(crate) use window::register_report_issue_action;
#[cfg(test)]
pub(crate) use window::{
    open_report, open_report_window_handle, register_report_issue_action_with,
};

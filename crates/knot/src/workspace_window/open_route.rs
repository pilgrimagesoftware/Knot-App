//! Which route opened a workspace window, for the application log (#458).

use std::fmt;

/// Where a request to open (or raise) a workspace window came from.
///
/// Logged with every open, so the log can tell a window the user opened
/// from one a scheduled start or a relaunch opened on its own - the
/// question #456 could not answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenRoute {
    /// The Workspace Manager.
    Manager,
    /// A Command Center card, heading or prompt.
    CommandCenter,
    /// A clicked desktop notification.
    Notification,
    /// The Library window's personas pane.
    Library,
    /// A scheduled start (#404).
    ScheduledStart,
    /// Reopening the windows that were open when Knot last quit.
    RestoreAtLaunch,
}

impl fmt::Display for OpenRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
                        Self::Manager => "manager",
                        Self::CommandCenter => "command-center",
                        Self::Notification => "notification",
                        Self::Library => "library",
                        Self::ScheduledStart => "scheduled-start",
                        Self::RestoreAtLaunch => "restore-at-launch",
                    })
    }
}

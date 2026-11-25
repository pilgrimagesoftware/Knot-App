//! Refresh and poll cadences, cache ages and key delays: how often something is
//! re-read or repainted and how stale it may get. Owns durations only;
//! geometry, colours and the agent-side schedules live in their own modules.

use std::time::Duration;

/// How often the workspace window's poll wakes to ask whether anything that
/// changed off the main thread needs a repaint.
///
/// ~30 Hz. The poll itself is cheap - a few map lookups and an `Instant`
/// compare; it only calls `cx.notify()` when a predicate says something
/// actually moved.
pub(crate) const REPAINT_POLL_INTERVAL: Duration = Duration::from_millis(33);

/// The longest a closing workspace window waits for its runtime's blocking
/// tasks before leaving them to finish on their own threads.
///
/// Long enough for the work that is nearly done - a `git` or `ps` call that
/// just returned - to land; short enough that a close never reads as a hang.
/// What it cuts off has nothing left to report to: the window is gone.
pub(crate) const WINDOW_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

/// How stale a cached `git diff --numstat` may get before the next render
/// asks for a fresh one.
///
/// `git` runs at most this often per agent no matter how often the window
/// repaints - see `workspace_window::sessions`.
pub(crate) const DIFF_STATS_MAX_AGE: Duration = Duration::from_secs(2);

/// How stale the git panel's working-tree status may get before the next
/// render asks for a fresh one.
///
/// Shorter than a diff stat's, because the panel is what the user acts
/// through: a stage they just made must appear promptly, and the watch that
/// would otherwise tell us is paused for exactly that window.
pub(crate) const GIT_STATUS_MAX_AGE: Duration = Duration::from_millis(500);

/// How stale a shown file diff may get before it is read again.
///
/// Long, because a diff is re-read when the selection changes or the status
/// is invalidated, not on a clock - this is a backstop against a diff that
/// somehow outlives both, not the mechanism that keeps it fresh.
pub(crate) const GIT_DIFF_MAX_AGE: Duration = Duration::from_secs(30);

/// How long text typed at a terminal agent's prompt gets to arrive before
/// the escape that dismisses its autocomplete, and how long the escape gets
/// before the Return. `TimingConstants.escapeKeyDelay` and `returnKeyDelay`:
/// sent back to back, an agent CLI's autocomplete takes the Return.
pub(crate) const TERMINAL_ESCAPE_KEY_DELAY: Duration = Duration::from_millis(300);
pub(crate) const TERMINAL_RETURN_KEY_DELAY: Duration = Duration::from_millis(300);

/// How stale a pull request's fetched state may get before the Pull Requests
/// view asks for it again.
///
/// Far longer than a diff stat's two seconds: this is a network round trip
/// through `gh`, and a pull request's title and status change on a human
/// timescale rather than a keystroke's. A list of twenty rows therefore costs
/// twenty `gh` runs a minute while it is open, and none while it is not.
pub(crate) const PULL_REQUEST_STATE_MAX_AGE: Duration = Duration::from_secs(60);

/// How long a merged pull request stays in the list after it merged.
///
/// Measured against the merge time the forge reports, not against when Knot
/// first saw the URL: a sighting says nothing about when the pull request
/// landed. A day is long enough that work merged this morning is still there
/// when the user looks after lunch, and short enough that the list is what is
/// outstanding rather than a lifetime tally.
///
/// A constant rather than a setting: a settings row, a persisted field and a
/// migration are a lot to spend on a number nobody has yet asked to change,
/// and this forecloses none of them.
pub(crate) const PULL_REQUEST_MERGED_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);

/// How stale the answer to "can state be fetched at all" may be.
///
/// The same minute as the rows it gates, and one `gh auth status` against
/// their twenty `gh pr view`s. Re-asking at all is what makes signing in
/// take effect while the window stays open; re-asking faster would spend a
/// subprocess to notice something that only changes when the user goes and
/// does it.
pub(crate) const FORGE_PROBE_MAX_AGE: Duration = Duration::from_secs(60);

/// How often the Changes view re-reads which repositories the workspace's
/// agents work in (#504). Cheap - three `git rev-parse`s per folder - and
/// only while the Issues or OpenSpec tab is shown, so a moved or new agent
/// folder is picked up within this.
pub(crate) const WORKSPACE_REPOS_MAX_AGE: Duration = Duration::from_secs(30);

/// How long a repository's open issues are shown before they are fetched
/// again (`workspace-issues`): one `gh` call per repository, so slower than
/// pull request state.
pub(crate) const ISSUES_MAX_AGE: Duration = Duration::from_secs(5 * 60);

/// How long a repository's OpenSpec change list is shown before it is read
/// again (`workspace-openspec-changes`). A directory read, so cheap; Refresh
/// now covers the change the user just made.
pub(crate) const OPENSPEC_CHANGES_MAX_AGE: Duration = Duration::from_secs(30);

/// How many folders below a working tree's root the OpenSpec scan looks for
/// another `openspec/changes/` (#203): enough for `services/api/` or
/// `packages/web/app/` in a monorepo, shallow enough that the walk stays a
/// handful of directory reads every `OPENSPEC_CHANGES_MAX_AGE`.
pub(crate) const OPENSPEC_SCAN_MAX_DEPTH: usize = 3;

/// Folders the OpenSpec scan never descends into: dependency and build
/// output trees, which are large and never hold a project's own changes.
/// Folders starting with `.` are skipped as well, without being listed.
pub(crate) const OPENSPEC_SCAN_SKIPPED_DIRS: &[&str] =
    &["node_modules", "target", "vendor", "build", "dist"];

/// How often the settings window drains the native font panel's selections.
///
/// The panel is an AppKit window with no callback into GPUI, so its choice is
/// picked up by polling a shared slot. macOS-only, because the panel is.
#[cfg(target_os = "macos")]
pub(crate) const FONT_PANEL_POLL_INTERVAL: Duration = Duration::from_millis(300);

/// How often the settings window re-reads the MCP server's state.
///
/// The supervisor publishes over a channel on its own thread, which GPUI
/// cannot await, so the MCP tab follows a change by polling the mirror. Far
/// slower than the repaint poll: a lifecycle transition is a human-scale
/// event, and the tick only notifies when the state differs from the one
/// last drawn.
pub(crate) const MCP_STATE_POLL_INTERVAL: Duration = Duration::from_millis(500);

//! Agent vocabulary and sidebar shortcuts: the default avatar, numbered
//! shortcut families, the key hint delay, the Changes row's count nouns and the
//! token estimate.

use std::time::Duration;

/// The avatar drawn for an agent that has none.
pub(crate) const DEFAULT_AGENT_AVATAR: &str = "🤖";

/// How many targets a numbered shortcut family reaches: the digits 1 to 9
/// (`keybindings`). The View menu's submenus list no more than this.
pub(crate) const NUMBERED_SHORTCUTS: usize = 9;

/// New Agent's fixed key, the Swift reference's. One definition, because the
/// sidebar's hint has to name the key the binding actually uses.
pub(crate) const NEW_AGENT_CHORD: &str = "cmd-t";

/// How long ⌘ has to be held before the sidebar shows its keys - long enough
/// that an ordinary ⌘C or ⌘Tab does not flash them (`agent-list-ui`).
pub(crate) const KEY_HINT_DELAY: Duration = Duration::from_millis(500);

/// How many assistant replies keep their parsed Markdown at once, the ones
/// nearest the reader's position (#246). Far more than a viewport and its
/// overdraw can hold - a reply is at least a line tall - so a reply on
/// screen is never the one dropped; small enough that a session thousands
/// of replies long does not keep every parsed document alive.
pub(crate) const PANEL_MARKDOWN_ROWS_KEPT: usize = 256;

/// A kind the sidebar's Changes row counts (#210), as catalog keys: its long
/// and short nouns, each singular then plural. Long while one or two kinds
/// are counted, short once all three are, so the line fits the sidebar.
pub(crate) struct CountNouns {
    pub(crate) long:  (&'static str, &'static str),
    pub(crate) short: (&'static str, &'static str),
}

pub(crate) const CHANGES_COUNT_PULL_REQUESTS: CountNouns =
    CountNouns { long:  ("changes_view.count.pull_request", "changes_view.count.pull_requests"),
                 short: ("changes_view.count.pr_short", "changes_view.count.prs_short"), };

pub(crate) const CHANGES_COUNT_ISSUES: CountNouns =
    CountNouns { long:  ("changes_view.count.issue", "changes_view.count.issues"),
                 short: ("changes_view.count.issue_short", "changes_view.count.issues_short"), };

pub(crate) const CHANGES_COUNT_OPENSPEC_CHANGES: CountNouns = CountNouns { long:  ("changes_view.count.openspec_change",
                                                                                   "changes_view.count.openspec_changes"),
                                                                           short: ("changes_view.count.change_short",
                                                                                   "changes_view.count.changes_short"), };

/// Characters to a token, for the working turn's estimate (#35): the usual
/// rule of thumb for English text and code.
pub(crate) const CHARS_PER_TOKEN: usize = 4;

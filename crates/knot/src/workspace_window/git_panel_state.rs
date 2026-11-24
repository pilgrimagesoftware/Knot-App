//! The git panel's window-held state, grouped out of `WorkspaceWindow`.
//!
//! Everything here is keyed by agent (or is a cache keyed by agent and path)
//! and exists only while that agent's panel is open. Grouped so the window
//! struct names one field for it; the field names are unchanged.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use gpui_kit::Entity;
use gpui_kit::ListState;
use gpui_kit::component::resizable::ResizableState;
use uuid::Uuid;

#[derive(Default)]
pub(super) struct GitPanelState {
    /// Agents whose git panel is open. Per-agent rather than a
    /// `WorkspaceViewMode`: the panel is scoped to one agent's folder and
    /// leaves that agent's content visible, so it is not a window mode.
    pub(super) git_panel_open:      BTreeSet<Uuid>,
    /// Last known working-tree status per agent with an open panel,
    /// refreshed off the render path - see `refresh_git_status`.
    pub(super) git_status:          crate::git_panel::state::GitStatusCache,
    /// Last known diff per selected row, keyed by `(agent, path, staged)`.
    ///
    /// The key is what removes the Swift race: a reply for a row the user
    /// has clicked away from lands in its own entry, and a render reads only
    /// the current selection's, so a late reply cannot overwrite the shown
    /// diff.
    pub(super) git_diffs:           crate::git_panel::state::GitDiffCache,
    /// Which row each open panel is showing a diff for.
    pub(super) git_selection:       BTreeMap<Uuid, crate::git_panel::state::Selection>,
    /// One working-tree watch per open panel. `Arc` because the watch's
    /// callback outlives the frame that started it.
    pub(super) git_watches:         BTreeMap<Uuid, Arc<knot_watch::Watch>>,
    /// The folder each agent's git status, diffs and diff stat were read
    /// from, so a change of folder drops what was read from the old one and
    /// moves the panel's watch (#25).
    pub(super) git_read_folders:    crate::read_folders::ReadFolders,
    /// Set by a watch callback, which runs on a tokio task with no GPUI
    /// context and so cannot touch the caches or notify. The repaint poll
    /// reads it, forgets the agent's status and redraws - the same
    /// off-main-thread hand-off `exited_sessions` uses.
    pub(super) git_watch_dirty:     BTreeMap<Uuid, Arc<AtomicBool>>,
    /// The panel's width per agent, in pixels. View state, not persisted:
    /// a reopened panel starts at the default again.
    pub(super) git_panel_width:     BTreeMap<Uuid, f32>,
    /// The last git operation that failed, per agent, so the panel can say
    /// so. Cleared by the next successful operation.
    pub(super) git_action_error:    BTreeMap<Uuid, String>,
    /// Staging operations in flight, per agent. `None` inside the slot means
    /// still running, so the poll can tell that from a finished success.
    /// Drained by `drain_git_actions`, which is what invalidates the caches
    /// and resumes the watch - the blocking task has no GPUI context.
    pub(super) pending_git_actions: BTreeMap<Uuid, super::git_panel::actions::GitActionSlot>,
    /// Commits in flight, per agent. Tracked here as well as in the commit
    /// window: the window shows the outcome, but the tree behind it is what
    /// has to be re-read, and the window cannot reach these caches.
    pub(super) pending_git_commits: BTreeMap<Uuid, crate::commit_window::CommitOutcome>,
    /// One virtualized diff list per agent with an open panel. One per agent
    /// rather than per selected file: only one diff is on screen at a time,
    /// so a second entry would be a leak rather than a cache.
    pub(super) git_diff_lists:      BTreeMap<Uuid, ListState>,
    /// The row count each `git_diff_lists` entry was last reconciled to, so a
    /// selection change splices rather than keeping measured heights against
    /// different content.
    pub(super) git_diff_row_counts: BTreeMap<Uuid, usize>,
    /// The divider between an agent's content and its git panel, one per
    /// agent with an open panel. Held here rather than keyed inside the
    /// element tree for the reason `sidebar_resize` is: the width is read
    /// outside the group too, to seed the panel's own size.
    pub(super) git_panel_resize:    BTreeMap<Uuid, Entity<ResizableState>>,
}

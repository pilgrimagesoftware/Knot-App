//! The Markdown state behind each assistant reply, kept for as long as the
//! conversation is, rather than for as long as the row is on screen (#246).
//!
//! A `TextView` built from text keeps its parsed document in keyed element
//! state, which GPUI drops on the first frame the element is not laid out.
//! The conversation list only lays out rows near the viewport, so a reply
//! that scrolled away lost its parse and came back empty: a reply over
//! `TextView`'s synchronous limit (4 KiB) is parsed on a background task,
//! so its first layout measured a fraction of its real height. The list
//! converts the next wheel event against that height, the parse lands, the
//! row grows - and everything below it moves by the difference. That was the
//! jump a reader saw scrolling back, idle or streaming.
//!
//! Owning the state here fixes the revisit: a reply that has been parsed
//! once keeps its height. A reply laid out for the first time can still
//! measure short while its parse runs, so each state is observed, and a
//! change to it marks its row for remeasuring. The list lays out a band of
//! [`super::render::LIST_OVERDRAW`] above the viewport, so the parse normally
//! lands and the row is remeasured before the reader scrolls it into view.
//!
//! What is kept is capped at [`PANEL_MARKDOWN_ROWS_KEPT`] replies, dropping
//! the ones farthest from where the reader is, so the retained state is
//! bounded by the reader's position rather than by how long the session has
//! run (`acp-panel-ui`, "Conversation rendering is virtualized"). A reply
//! dropped that way comes back like one never laid out: parsed again, and
//! remeasured as above.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Entity;
use gpui_kit::ListState;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::component::text::TextViewState;

use crate::consts::PANEL_MARKDOWN_ROWS_KEPT;

/// One reply's parsed Markdown and the observation that remeasures its row.
struct MarkdownRow {
    state:      Entity<TextViewState>,
    _remeasure: Subscription,
}

/// Every assistant reply's Markdown state, keyed by message index.
///
/// Cheap to clone: clones share one map, so the list's row callback and the
/// pane that owns the list see the same states.
#[derive(Clone, Default)]
pub(crate) struct MarkdownRows(Rc<RefCell<BTreeMap<usize, MarkdownRow>>>);

impl MarkdownRows {
    /// The state rendering message `index`, created on first use and brought
    /// up to date with `text` on every call after.
    ///
    /// Called from the list's row callback, so `window` is mid-layout: a new
    /// state's observer notifies the view being drawn, as GPUI's own keyed
    /// state does, and remeasures the row in `list`.
    pub(super) fn state(&self, index: usize, text: &str, list: &ListState, window: &Window,
                        cx: &mut App)
                        -> Entity<TextViewState> {
        // Cloned out so the map is not borrowed while the state updates.
        let known = self.0.borrow().get(&index).map(|row| row.state.clone());
        if let Some(state) = known {
            state.update(cx, |state, cx| state.set_text(text, cx));
            return state;
        }
        let state = cx.new(|cx| TextViewState::markdown(text, cx));
        let list = list.clone();
        let view = window.current_view();
        let remeasure = cx.observe(&state, move |_, cx| {
                              list.remeasure_items(index..index + 1);
                              cx.notify(view);
                          });
        self.0.borrow_mut().insert(index,
                                   MarkdownRow { state:      state.clone(),
                                                 _remeasure: remeasure, });
        state
    }

    /// Brings the kept states in line with a conversation of `len`
    /// messages read from around row `anchor`.
    ///
    /// Forgets every message at or past `len`: a reloaded session replays
    /// into a fresh `PanelState`, and the index a stale state sits at can
    /// come back as a different message. Then, past
    /// [`PANEL_MARKDOWN_ROWS_KEPT`], forgets the replies farthest from
    /// `anchor` - farthest rather than by a fixed index window, because a
    /// run of compacted tool calls is many zero-height rows, and a window
    /// counted in rows could drop a reply that is on screen.
    pub(super) fn retain(&self, len: usize, anchor: usize) {
        let mut rows = self.0.borrow_mut();
        if rows.last_key_value().is_some_and(|(&last, _)| last >= len) {
            drop(rows.split_off(&len));
        }
        if rows.len() <= PANEL_MARKDOWN_ROWS_KEPT {
            return;
        }
        let mut by_distance: Vec<usize> = rows.keys().copied().collect();
        by_distance.sort_unstable_by_key(|&index| index.abs_diff(anchor));
        for index in &by_distance[PANEL_MARKDOWN_ROWS_KEPT..] {
            rows.remove(index);
        }
    }
}

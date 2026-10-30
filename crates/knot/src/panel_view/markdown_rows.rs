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
use gpui_kit::component::text::{RangeHighlight, RenderedText, TextViewState};

use super::find::{FindHighlight, FindMatch};
use crate::consts::PANEL_MARKDOWN_ROWS_KEPT;

/// One reply's parsed Markdown and the observation that remeasures its row.
struct MarkdownRow {
    state:       Entity<TextViewState>,
    /// What the row's find highlights were last set for, so a frame that
    /// changes nothing sets nothing: setting them notifies the state, whose
    /// observer redraws - which would set them again.
    highlighted: Option<Highlighted>,
    _remeasure:  Subscription,
}

/// The find a row's highlights were computed for (#251).
#[derive(PartialEq)]
struct Highlighted {
    generation: u64,
    current:    Option<usize>,
    text:       RenderedText,
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
                                   MarkdownRow { state:       state.clone(),
                                                 highlighted: None,
                                                 _remeasure:  remeasure, });
        state
    }

    /// Paints `find`'s matches in message `index`'s reply, or clears them
    /// when no find is open. Its ranges come from the text the reply
    /// renders, which is what highlights index (see `find`).
    pub(super) fn highlight(&self, index: usize, find: Option<&FindHighlight>, cx: &mut App) {
        let mut rows = self.0.borrow_mut();
        let Some(row) = rows.get_mut(&index)
        else {
            return;
        };
        let text = row.state.read(cx).rendered_text();
        let wanted =
            find.map(|find| Highlighted { generation: find.generation,
                                          current:
                                              find.current
                                                  .filter(|current| current.message == index)
                                                  .map(|current| current.nth),
                                          text:       text.clone(), });
        if wanted == row.highlighted {
            return;
        }
        let state = row.state.clone();
        row.highlighted = wanted;
        // Released before the update: the state's observer re-enters the
        // list, which reads this map.
        drop(rows);
        state.update(cx, |state, cx| match find {
                 Some(find) => {
                     let ranges = find.ranges(index, text.as_str())
                                      .into_iter()
                                      .map(|(range, color)| RangeHighlight::new(range, color));
                     // Computed from the text just read, so the ranges are
                     // in bounds; a rejected set keeps the old one.
                     let _ = state.set_range_highlights(ranges, cx);
                 }
                 None => state.clear_range_highlights(cx),
             });
    }

    /// Scrolls the line holding `found` into view inside its reply, once
    /// the reply is laid out. Nothing for a reply with no state yet, or a
    /// match its rendered text does not have.
    pub(crate) fn reveal(&self, found: FindMatch, find: &FindHighlight, cx: &mut App) {
        let Some(state) = self.0
                              .borrow()
                              .get(&found.message)
                              .map(|row| row.state.clone())
        else {
            return;
        };
        let text = state.read(cx).rendered_text();
        let Some(range) = find.query
                              .find_all(text.as_str())
                              .into_iter()
                              .nth(found.nth)
        else {
            return;
        };
        state.update(cx, |state, cx| {
                 // Best effort, as `reveal_range` itself is.
                 let _ = state.reveal_range(range, cx);
             });
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

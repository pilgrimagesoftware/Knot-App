//! A conversation's virtualized list, together with what its rows keep
//! between frames.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::ListAlignment;
use gpui_kit::ListState;
use gpui_kit::px;

use super::find::FindHighlight;
use super::markdown_rows::MarkdownRows;
use super::render::LIST_OVERDRAW;

/// The list one agent's conversation scrolls in, and the Markdown state of
/// its replies, which has to live exactly as long as the list does: it is
/// keyed by the same row indices, and outliving the list would keep parsed
/// documents for a conversation nobody can see.
///
/// Clones share both.
#[derive(Clone)]
pub(crate) struct PanelList {
    pub(crate) list:     ListState,
    pub(crate) markdown: MarkdownRows,
    /// The open find's highlights (#251), set by the pane each frame before
    /// the list draws, and read by the rows it lays out. `None` while no
    /// find is open.
    pub(crate) find:     Rc<RefCell<Option<FindHighlight>>>,
}

impl PanelList {
    /// An empty conversation; the caller reconciles its item count each
    /// frame (see [`super::sync_row_count`]).
    pub(crate) fn new() -> Self {
        Self { list:     ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW)),
               markdown: MarkdownRows::default(),
               find:     Rc::default(), }
    }
}

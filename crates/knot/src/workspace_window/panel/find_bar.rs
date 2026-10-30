//! The conversation's find bar (#251): ⌘F opens it above an agent's
//! conversation, and it finds what is typed - as text, or as a regular
//! expression - across the prompts and replies, highlights every match and
//! steps between them.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`, "Finding text in the
//! conversation".
//!
//! What a query matches and what the rows paint is `panel_view::find`. This
//! module owns the bar: its field and toggle, the match list, which match is
//! current, and taking the conversation to it. The rows learn about the find
//! through the agent's `PanelList`, which the pane updates each frame from
//! [`WorkspaceWindow::sync_panel_find`].

use std::rc::Rc;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ListOffset;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Selectable;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Enter;
use gpui_kit::component::input::Escape;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::rgb;
use uuid::Uuid;

use crate::consts::{FIND_CURRENT_ALPHA, FIND_CURRENT_COLOR, FIND_MATCH_ALPHA, FIND_MATCH_COLOR};
use crate::panel_state::PanelState;
use crate::panel_view::{
    FindColors, FindHighlight, FindMatch, FindMode, FindQuery, PanelList, conversation_matches,
};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::element_key;

/// One agent's open find.
pub(in crate::workspace_window) struct PanelFind {
    input:      Entity<InputState>,
    mode:       FindMode,
    /// The text and mode `query` was compiled from.
    compiled:   Option<(String, FindMode)>,
    query:      Query,
    /// Bumped with every new query, so the rows can tell theirs is stale.
    generation: u64,
    matches:    Vec<FindMatch>,
    /// Index into `matches`.
    current:    Option<usize>,
    /// What `matches` was searched over, so an unchanged conversation is
    /// not searched again every frame.
    searched:   Option<Searched>,
    /// Set by a new query or a step: the next frame takes the conversation
    /// to the current match.
    reveal:     bool,
    _changes:   Subscription,
}

/// The compiled query, or why there is none.
enum Query {
    Empty,
    Invalid,
    Ready(Rc<FindQuery>),
}

/// A conversation as far as find is concerned: streaming only ever grows
/// its last message, so the count and that message's length tell a change.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Searched {
    generation: u64,
    messages:   usize,
    last_len:   usize,
}

impl Searched {
    fn of(state: &PanelState, generation: u64) -> Self {
        let last_len = state.messages
                            .last()
                            .and_then(crate::panel_view::find_searched_text)
                            .map_or(0, str::len);
        Self { generation,
               messages: state.messages.len(),
               last_len }
    }
}

/// What one frame's [`WorkspaceWindow::sync_panel_find`] found.
pub(in crate::workspace_window) struct FindFrame {
    pub(in crate::workspace_window) status:   FindStatus,
    /// The conversation was just taken to the current match, so it should
    /// stop following new output. Reported rather than done: stopping it
    /// locks the conversation's state, which the caller holds while this
    /// runs.
    pub(in crate::workspace_window) revealed: bool,
}

/// What the bar shows beside the field.
pub(in crate::workspace_window) enum FindStatus {
    Empty,
    Invalid,
    NoMatches,
    At { current: usize, total: usize },
}

impl WorkspaceWindow {
    /// Opens `id`'s find bar, or returns to it, with its text selected so
    /// typing replaces it.
    pub(in crate::workspace_window) fn open_panel_find(&mut self, id: Uuid, window: &mut Window,
                                                       cx: &mut Context<Self>) {
        if let std::collections::btree_map::Entry::Vacant(slot) = self.panel_finds.entry(id) {
            let input = cx.new(|cx| {
                              InputState::new(window, cx)
                    .placeholder(knot_core::l10n::t("panel.find.placeholder"))
                          });
            let changes = cx.subscribe_in(&input, window, |_, _, event, _, cx| {
                                if matches!(event, InputEvent::Change) {
                                    cx.notify();
                                }
                            });
            slot.insert(PanelFind { input,
                                    mode: FindMode::default(),
                                    compiled: None,
                                    query: Query::Empty,
                                    generation: 0,
                                    matches: Vec::new(),
                                    current: None,
                                    searched: None,
                                    reveal: false,
                                    _changes: changes });
        }
        if let Some(find) = self.panel_finds.get(&id) {
            find.input.update(cx, |state, cx| {
                          state.focus(window, cx);
                          state.select_all(window, cx);
                      });
        }
        cx.notify();
    }

    /// Closes `id`'s find bar, takes its highlights down, and hands focus
    /// back to the composer.
    pub(in crate::workspace_window) fn close_panel_find(&mut self, id: Uuid,
                                                        window: &mut Window,
                                                        cx: &mut Context<Self>) {
        if self.panel_finds.remove(&id).is_none() {
            return;
        }
        if let Some(panel) = self.panel_lists.get(&id) {
            panel.find.borrow_mut().take();
        }
        if let Some(input) = self.panel_prompt_inputs.get(&id) {
            input.update(cx, |state, cx| state.focus(window, cx));
        }
        cx.notify();
    }

    /// Switches `id`'s find between text and regular expression.
    pub(in crate::workspace_window) fn toggle_panel_find_mode(&mut self, id: Uuid,
                                                              cx: &mut Context<Self>) {
        if let Some(find) = self.panel_finds.get_mut(&id) {
            find.mode = match find.mode {
                FindMode::Substring => FindMode::Regex,
                FindMode::Regex => FindMode::Substring,
            };
            cx.notify();
        }
    }

    /// Moves `id`'s current match on by one, or back, wrapping at either
    /// end, and takes the conversation to it.
    pub(in crate::workspace_window) fn step_panel_find(&mut self, id: Uuid, forward: bool,
                                                       cx: &mut Context<Self>) {
        let Some(find) = self.panel_finds.get_mut(&id)
        else {
            return;
        };
        let total = find.matches.len();
        if total == 0 {
            return;
        }
        find.current = Some(match (find.current, forward) {
                                (None, true) => 0,
                                (None, false) => total - 1,
                                (Some(at), true) => (at + 1) % total,
                                (Some(at), false) => (at + total - 1) % total,
                            });
        find.reveal = true;
        cx.notify();
    }

    /// Brings `id`'s find up to date with its field and the conversation,
    /// hands the rows what to paint through `panel`, and takes the
    /// conversation to the current match when a query or a step asked for
    /// it. Called by the pane every frame before the list draws; a frame
    /// where nothing changed recompiles and searches nothing.
    pub(in crate::workspace_window) fn sync_panel_find(&mut self, id: Uuid, state: &PanelState,
                                                       panel: &PanelList, window: &mut Window,
                                                       cx: &mut Context<Self>)
                                                       -> Option<FindFrame> {
        let Some(find) = self.panel_finds.get_mut(&id)
        else {
            panel.find.borrow_mut().take();
            return None;
        };
        let text = find.input.read(cx).value().to_string();
        if find.compiled.as_ref() != Some(&(text.clone(), find.mode)) {
            find.query = match FindQuery::new(&text, find.mode) {
                Ok(Some(query)) => Query::Ready(Rc::new(query)),
                Ok(None) => Query::Empty,
                Err(_) => Query::Invalid,
            };
            find.compiled = Some((text, find.mode));
            find.generation += 1;
            find.current = None;
            find.reveal = true;
        }
        let Query::Ready(query) = &find.query
        else {
            find.matches.clear();
            panel.find.borrow_mut().take();
            let status = match find.query {
                Query::Invalid => FindStatus::Invalid,
                Query::Empty | Query::Ready(_) => FindStatus::Empty,
            };
            return Some(FindFrame { status,
                                    revealed: false });
        };
        let searched = Searched::of(state, find.generation);
        if find.searched != Some(searched) {
            find.matches = conversation_matches(&state.messages, query);
            find.searched = Some(searched);
        }
        if find.current.is_none() && !find.matches.is_empty() {
            // A new query starts from the first match at or below what is
            // on screen, as a browser's find does, rather than from the top.
            let top = panel.list.logical_scroll_top().item_ix;
            find.current = Some(find.matches
                                    .iter()
                                    .position(|found| found.message >= top)
                                    .unwrap_or(0));
        }
        if let Some(at) = find.current {
            find.current = Some(at.min(find.matches.len().saturating_sub(1)));
        }
        let current = find.current.and_then(|at| find.matches.get(at).copied());
        let highlight = FindHighlight { query: Rc::clone(query),
                                        generation: find.generation,
                                        current,
                                        colors: find_colors() };
        let revealed = std::mem::take(&mut find.reveal) && current.is_some();
        if revealed && let Some(found) = current {
            reveal_match(panel, found, &highlight, window);
        }
        *panel.find.borrow_mut() = Some(highlight);
        let status = match find.current {
            Some(at) => FindStatus::At { current: at + 1,
                                         total:   find.matches.len(), },
            None => FindStatus::NoMatches,
        };
        Some(FindFrame { status, revealed })
    }

    /// Stops `id`'s conversation following new output, so it stays on the
    /// match it was taken to - as scrolling there by hand would. Not while
    /// the conversation's state is locked: this locks it.
    pub(in crate::workspace_window) fn stop_following(&self, id: Uuid) {
        if let Some(slot) = self.panel_sessions.get(&id)
           && let crate::panel_session::PanelSessionSlot::Ready(handle) = &*slot.lock()
        {
            handle.clear_tracking();
        }
    }

    /// The find bar over `id`'s conversation, or `None` while it is closed.
    pub(in crate::workspace_window) fn render_panel_find(&self, id: Uuid, status: FindStatus,
                                                         cx: &mut Context<Self>)
                                                         -> Option<gpui_kit::AnyElement> {
        let find = self.panel_finds.get(&id)?;
        let key = element_key(id);
        let theme = cx.theme();
        let (label, color) = match status {
            FindStatus::Empty => (String::new(), theme.muted_foreground),
            FindStatus::Invalid => (knot_core::l10n::t("panel.find.invalid"), theme.danger),
            FindStatus::NoMatches => {
                (knot_core::l10n::t("panel.find.no_matches"), theme.muted_foreground)
            }
            FindStatus::At { current, total } => {
                (knot_core::l10n::t_with("panel.find.count",
                                         &[("current", &current.to_string()),
                                           ("total", &total.to_string())]),
                 theme.muted_foreground)
            }
        };
        let border = theme.border;
        let regex = find.mode == FindMode::Regex;
        let entity = cx.entity();
        let button = |name: &'static str, icon: IconName, tooltip: &str| {
            Button::new((name, key)).icon(icon)
                                    .tooltip(knot_core::l10n::t(tooltip))
                                    .ghost()
                                    .small()
        };
        Some(h_flex().id(("panel-find", key))
                     .debug_selector(|| "panel-find-bar".into())
                     .w_full()
                     .flex_shrink_0()
                     .gap_1()
                     .px_2()
                     .py_1()
                     .items_center()
                     .border_b_1()
                     .border_color(border)
                     .capture_action::<Enter>({
                         let entity = entity.clone();
                         move |enter, _, app| {
                             let forward = !enter.shift;
                             entity.update(app, |view, cx| {
                                       view.step_panel_find(id, forward, cx);
                                       cx.stop_propagation();
                                   });
                         }
                     })
                     .capture_action::<Escape>({
                         let entity = entity.clone();
                         move |_, window, app| {
                             entity.update(app, |view, cx| {
                                       view.close_panel_find(id, window, cx);
                                       cx.stop_propagation();
                                   });
                         }
                     })
                     .child(div().flex_1()
                                 .min_w_0()
                                 .child(Input::new(&find.input).small()))
                     .child(Button::new(("panel-find-regex", key)).label(".*")
                                                                 .tooltip(knot_core::l10n::t("panel.find.regex"))
                                                                 .small()
                                                                 .selected(regex)
                                                                 .on_click({
                                                                     let entity = entity.clone();
                                                                     move |_, _, app| {
                                                                         entity.update(app, |view, cx| {
                                                                             view.toggle_panel_find_mode(id, cx)
                                                                         });
                                                                     }
                                                                 }))
                     .child(div().flex_shrink_0()
                                 .min_w(px(64.))
                                 .text_xs()
                                 .text_color(color)
                                 .debug_selector(|| "panel-find-status".into())
                                 .child(label))
                     .child(button("panel-find-previous",
                                   IconName::ChevronUp,
                                   "panel.find.previous").on_click({
                                                             let entity = entity.clone();
                                                             move |_, _, app| {
                                                                 entity.update(app, |view, cx| {
                                                                     view.step_panel_find(id, false, cx)
                                                                 });
                                                             }
                                                         }))
                     .child(button("panel-find-next", IconName::ChevronDown, "panel.find.next")
                         .on_click({
                             let entity = entity.clone();
                             move |_, _, app| {
                                 entity.update(app, |view, cx| view.step_panel_find(id, true, cx));
                             }
                         }))
                     .child(button("panel-find-close", IconName::Close, "panel.find.close")
                         .on_click(move |_, window, app| {
                             entity.update(app, |view, cx| view.close_panel_find(id, window, cx));
                         }))
                     .into_any_element())
    }
}

#[cfg(test)]
impl WorkspaceWindow {
    /// `id`'s current match (from 0) and how many there are, for tests.
    pub(in crate::workspace_window) fn panel_find_position(&self, id: Uuid)
                                                           -> Option<(Option<usize>, usize)> {
        self.panel_finds
            .get(&id)
            .map(|find| (find.current, find.matches.len()))
    }

    /// `id`'s find field, for tests to type into.
    pub(in crate::workspace_window) fn panel_find_input(&self, id: Uuid)
                                                        -> Option<Entity<InputState>> {
        self.panel_finds.get(&id).map(|find| find.input.clone())
    }
}

/// The find colours, translucent over whatever the conversation is drawn on.
fn find_colors() -> FindColors {
    FindColors { matched: gpui_kit::Hsla::from(rgb(FIND_MATCH_COLOR)).opacity(FIND_MATCH_ALPHA),
                 current: gpui_kit::Hsla::from(rgb(FIND_CURRENT_COLOR)).opacity(FIND_CURRENT_ALPHA), }
}

/// Scrolls the conversation to `found`'s message, then - once that row is
/// laid out - to the line inside a reply that holds it.
fn reveal_match(panel: &PanelList, found: FindMatch, highlight: &FindHighlight,
                window: &mut Window) {
    panel.list
         .scroll_to(ListOffset { item_ix:        found.message,
                                 offset_in_item: px(0.), });
    let markdown = panel.markdown.clone();
    let highlight = highlight.clone();
    window.on_next_frame(move |_, cx| markdown.reveal(found, &highlight, cx));
}

#[cfg(test)]
mod tests;

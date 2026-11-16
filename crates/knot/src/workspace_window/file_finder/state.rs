//! The finder's state and what it does: open, search, move, choose, close.

use std::time::Duration;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use uuid::Uuid;

use crate::consts::FILE_FINDER_DEBOUNCE_MS;
use crate::file_search::FileSearch;
use crate::panel_commands::LookupMatch;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::mentions::MentionState;

/// The open finder. One per window: it belongs to the selected agent and
/// closes when another is selected.
pub(in crate::workspace_window) struct FileFinder {
    pub(super) agent:    Uuid,
    pub(super) input:    Entity<InputState>,
    pub(super) search:   FileSearch,
    pub(super) selected: usize,
    _changes:            Subscription,
}

/// Where a chosen path is delivered; see `WorkspaceWindow::found_path_target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace_window) enum FoundPathTarget {
    Composer,
    Terminal,
    Nowhere,
}

/// What is typed into a terminal for a chosen `path`: the path alone - no
/// newline, so it waits at the prompt rather than running (`file-finder`,
/// "Choosing a file"), and no quoting, as the reference typed it.
pub(in crate::workspace_window) fn terminal_text(path: &str) -> String {
    path.to_owned()
}

/// What the finder lists below its field.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::workspace_window) enum FinderRows {
    /// Nothing typed, nothing to say.
    Empty,
    /// The agent's folder is still being listed.
    Gathering,
    /// Nothing typed, and the folder was larger than the listing's cap.
    TooManyFiles,
    /// The ranking for what is typed has not landed yet.
    Matching,
    /// A non-empty query matched nothing.
    NoMatches,
    Results(Vec<LookupMatch>),
}

impl WorkspaceWindow {
    /// Opens the finder over the selected agent's folder (Edit > Find File…),
    /// focusing its field. Starts the agent's listing if it has none.
    pub(in crate::workspace_window) fn open_file_finder(&mut self, window: &mut Window,
                                                        cx: &mut Context<Self>) {
        let Some(agent) = self.selected_agent
        else {
            return;
        };
        self.ensure_panel_mentions(agent);
        let input = cx.new(|cx| {
                          InputState::new(window, cx)
                    .placeholder(knot_core::l10n::t("finder.placeholder"))
                      });
        // A changed query moves the selection back to the top, as the
        // reference did when its results arrived.
        let changes = cx.subscribe_in(&input, window, |view, _, event, _, cx| {
                            if matches!(event, InputEvent::Change)
                               && let Some(finder) = view.file_finder.as_mut()
                            {
                                finder.selected = 0;
                                cx.notify();
                            }
                        });
        input.update(cx, |state, cx| state.focus(window, cx));
        self.file_finder = Some(FileFinder { agent,
                                             input,
                                             search: FileSearch::default(),
                                             selected: 0,
                                             _changes: changes });
        cx.notify();
    }

    /// Closes the finder and gives focus back to the agent's composer when
    /// it has one; nothing is delivered.
    pub(in crate::workspace_window) fn close_file_finder(&mut self, window: &mut Window,
                                                         cx: &mut Context<Self>) {
        let Some(finder) = self.file_finder.take()
        else {
            return;
        };
        if let Some(input) = self.panel_prompt_inputs.get(&finder.agent) {
            input.update(cx, |state, cx| state.focus(window, cx));
        }
        cx.notify();
    }

    /// What the finder lists for its current query. Called from render, so
    /// it never ranks: it asks the search, which ranks off the render path
    /// 150 ms after the query last changed, and reads what has landed.
    pub(in crate::workspace_window) fn file_finder_rows(&mut self, cx: &Context<Self>)
                                                        -> FinderRows {
        let Some(finder) = self.file_finder.as_ref()
        else {
            return FinderRows::Empty;
        };
        let (agent, query) = (finder.agent, finder.input.read(cx).value().to_string());
        let handle = self.runtime.handle().clone();
        let Some(mentions) = self.panel_mentions.get(&agent)
        else {
            return FinderRows::Gathering;
        };
        let (state, entries) = (mentions.state(), std::sync::Arc::clone(mentions.entries()));
        let Some(finder) = self.file_finder.as_mut()
        else {
            return FinderRows::Empty;
        };
        match state {
            MentionState::Gathering => FinderRows::Gathering,
            MentionState::Ready { overflowed } if query.is_empty() => {
                if overflowed {
                    FinderRows::TooManyFiles
                }
                else {
                    FinderRows::Empty
                }
            }
            MentionState::Ready { .. } => {
                finder.search.request(&handle,
                                      &entries,
                                      &query,
                                      Duration::from_millis(FILE_FINDER_DEBOUNCE_MS));
                match finder.search.matches_for(&query) {
                    None => FinderRows::Matching,
                    Some([]) => FinderRows::NoMatches,
                    Some(matches) => FinderRows::Results(matches.to_vec()),
                }
            }
        }
    }

    /// Moves the selection by `delta`, stopping at either end of `count`
    /// results.
    pub(in crate::workspace_window) fn move_file_finder(&mut self, delta: isize, count: usize) {
        if let Some(finder) = self.file_finder.as_mut()
           && count > 0
        {
            let last = count as isize - 1;
            finder.selected = (finder.selected as isize + delta).clamp(0, last) as usize;
        }
    }

    /// Chooses result `index` (the selection when `None`): closes the
    /// finder and hands the path to its agent.
    pub(in crate::workspace_window) fn choose_file_finder(&mut self, index: Option<usize>,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>) {
        let FinderRows::Results(rows) = self.file_finder_rows(cx)
        else {
            return;
        };
        let Some(finder) = self.file_finder.as_ref()
        else {
            return;
        };
        let agent = finder.agent;
        let Some(row) = rows.get(index.unwrap_or(finder.selected))
        else {
            return;
        };
        let path = row.entry.token.clone();
        self.close_file_finder(window, cx);
        self.deliver_found_path(agent, &path, window, cx);
    }

    /// Hands `path` to `agent`: inserted at the composer's caret for a
    /// Panel-mode agent, typed at the prompt (no newline) for a
    /// Terminal-mode one. Plain text in both - no `@`, no escaping - and
    /// not copied to the clipboard (`file-finder`, "Choosing a file").
    pub(in crate::workspace_window) fn deliver_found_path(&mut self, agent: Uuid, path: &str,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>) {
        match self.found_path_target(agent) {
            FoundPathTarget::Composer => {
                // Created if the agent has none yet - a Panel agent whose
                // conversation was never shown - so the path is not dropped.
                let input = self.panel_prompt_input(agent, window, cx);
                input.update(cx, |state, cx| {
                         state.replace(path, window, cx);
                         state.focus(window, cx);
                     });
            }
            FoundPathTarget::Terminal => {
                if let Some(session) = self.sessions.get(&agent)
                   && let Err(error) = session.lock().send_text(&terminal_text(path))
                {
                    eprintln!("failed to type a found path for agent {agent}: {error}");
                }
            }
            FoundPathTarget::Nowhere => {}
        }
    }

    /// Where a found path for `agent` goes: a Panel-mode agent's composer, a
    /// Terminal-mode agent's terminal, or nowhere for an agent that is gone.
    pub(in crate::workspace_window) fn found_path_target(&self, agent: Uuid) -> FoundPathTarget {
        match self.store.lock().agent(agent).map(|agent| agent.view_mode) {
            Some(knot_core::ViewMode::Panel) => FoundPathTarget::Composer,
            Some(knot_core::ViewMode::Terminal) => FoundPathTarget::Terminal,
            None => FoundPathTarget::Nowhere,
        }
    }

    /// Moves a landed ranking into the finder's view, and closes the finder
    /// once its agent is no longer the selected one. Says whether anything
    /// changed, for the repaint poll.
    pub(in crate::workspace_window) fn drain_file_finder(&mut self) -> bool {
        let Some(finder) = self.file_finder.as_mut()
        else {
            return false;
        };
        if self.selected_agent != Some(finder.agent) {
            self.file_finder = None;
            return true;
        }
        finder.search.drain()
    }

    /// Forgets the finder's ranking for `agent`, whose listing changed.
    pub(in crate::workspace_window) fn invalidate_file_finder(&mut self, agent: Uuid) {
        if let Some(finder) = self.file_finder.as_mut()
           && finder.agent == agent
        {
            finder.search.invalidate();
        }
    }
}

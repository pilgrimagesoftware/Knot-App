//! Finding text in a conversation (#251): what a find query matches, which
//! messages it searches, and what the rows draw for its matches.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`, "Finding text in the
//! conversation".
//!
//! The find bar itself is the pane's (`workspace_window::panel::find_bar`);
//! this module is the part the rows need, kept free of the window so it can
//! be tested as plain functions.
//!
//! # Where matches are counted and where they are painted
//!
//! The prompts and replies a user reads are searched. Tool calls, shell runs
//! and notices are not: they are cards, not conversation text.
//!
//! A match is counted in the message text as the agent sent it. A reply is
//! Markdown, and painting a highlight needs offsets into the text it renders,
//! which exists only for the replies laid out near the viewport (see
//! `markdown_rows`). So a reply's highlights come from running the same query
//! over its rendered text, and its *n*th counted match is painted as its *n*th
//! rendered one. The two agree except where the query matches Markdown syntax
//! itself - `**` counts a match that has nothing to paint.

use std::ops::Range;
use std::rc::Rc;

use gpui_kit::AnyElement;
use gpui_kit::HighlightStyle;
use gpui_kit::Hsla;
use gpui_kit::IntoElement;
use gpui_kit::StyledText;
use regex::Regex;
use regex::RegexBuilder;

use crate::consts::FIND_REGEX_SIZE_LIMIT;
use crate::panel_state::PanelMessage;

/// How a find query's text is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FindMode {
    /// The text as typed, anywhere in a message.
    #[default]
    Substring,
    /// A regular expression (`regex` crate syntax).
    Regex,
}

/// A compiled find query. Case is ignored in both modes; a regular
/// expression can turn that off itself with `(?-i)`.
#[derive(Debug, Clone)]
pub(crate) struct FindQuery {
    matcher: Regex,
}

impl FindQuery {
    /// Compiles `text` in `mode`: `Ok(None)` for an empty query, which finds
    /// nothing, and `Err` for a regular expression that does not parse.
    pub(crate) fn new(text: &str, mode: FindMode) -> Result<Option<Self>, regex::Error> {
        if text.is_empty() {
            return Ok(None);
        }
        // A substring is an escaped regular expression, so both modes share
        // one matcher and one notion of "ignoring case" - and offsets always
        // index the original text, which lowercasing it would not promise.
        let pattern = match mode {
            FindMode::Substring => regex::escape(text),
            FindMode::Regex => text.to_owned(),
        };
        let matcher = RegexBuilder::new(&pattern).case_insensitive(true)
                                                 .size_limit(FIND_REGEX_SIZE_LIMIT)
                                                 .build()?;
        Ok(Some(Self { matcher }))
    }

    /// Every match in `haystack`, as byte ranges in order. A match of no
    /// characters - `a*` against `b` - is not one: there is nothing to show.
    pub(crate) fn find_all(&self, haystack: &str) -> Vec<Range<usize>> {
        self.matcher
            .find_iter(haystack)
            .filter(|found| !found.is_empty())
            .map(|found| found.range())
            .collect()
    }
}

/// One match: the `nth` (from 0) in message `message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FindMatch {
    pub(crate) message: usize,
    pub(crate) nth:     usize,
}

/// The text find searches in `message`, or `None` for a message it skips.
pub(crate) fn searched_text(message: &PanelMessage) -> Option<&str> {
    match message {
        PanelMessage::User(text) | PanelMessage::Assistant(text) => Some(text),
        PanelMessage::ToolCall(_)
        | PanelMessage::Shell(_)
        | PanelMessage::Error(_)
        | PanelMessage::Notice(_) => None,
    }
}

/// Every match of `query` across `messages`, in conversation order.
pub(crate) fn conversation_matches(messages: &[PanelMessage], query: &FindQuery) -> Vec<FindMatch> {
    messages.iter()
            .enumerate()
            .filter_map(|(message, item)| searched_text(item).map(|text| (message, text)))
            .flat_map(|(message, text)| {
                (0..query.find_all(text).len()).map(move |nth| FindMatch { message, nth })
            })
            .collect()
}

/// What the rows paint for an open find: every match, and the current one
/// in its own colour.
#[derive(Clone)]
pub(crate) struct FindHighlight {
    pub(crate) query:      Rc<FindQuery>,
    /// Bumped whenever `query` changes, so a row can tell its highlights are
    /// stale without comparing queries.
    pub(crate) generation: u64,
    pub(crate) current:    Option<FindMatch>,
    pub(crate) colors:     FindColors,
}

/// A match's background, and the current match's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FindColors {
    pub(crate) matched: Hsla,
    pub(crate) current: Hsla,
}

impl FindHighlight {
    /// The ranges to paint in message `message`'s `text`, each with its
    /// background.
    pub(crate) fn ranges(&self, message: usize, text: &str) -> Vec<(Range<usize>, Hsla)> {
        let current = self.current
                          .filter(|current| current.message == message)
                          .map(|current| current.nth);
        self.query
            .find_all(text)
            .into_iter()
            .enumerate()
            .map(|(nth, range)| {
                let color = if Some(nth) == current {
                    self.colors.current
                }
                else {
                    self.colors.matched
                };
                (range, color)
            })
            .collect()
    }
}

/// A prompt's plain `text` (message `message`), with `find`'s matches in
/// it painted. A reply's go through its Markdown state instead.
pub(super) fn highlighted_text(text: &str, message: usize, find: Option<&FindHighlight>)
                               -> AnyElement {
    let highlights: Vec<_> = find.map(|find| find.ranges(message, text))
                                 .unwrap_or_default()
                                 .into_iter()
                                 .map(|(range, color)| {
                                     (range,
                                      HighlightStyle { background_color: Some(color),
                                                       ..Default::default() })
                                 })
                                 .collect();
    if highlights.is_empty() {
        return text.to_owned().into_any_element();
    }
    StyledText::new(text.to_owned()).with_highlights(highlights)
                                    .into_any_element()
}

#[cfg(test)]
mod tests;

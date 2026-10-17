//! A checklist an agent shows in its artifact panel (#301): questions for
//! the user to answer and items to tick, kept where the user can work
//! through them instead of in a conversation that scrolls them away.
//!
//! The agent writes the items (`show-checklist`), the user writes the
//! answers (the panel), and either side reads the whole (`get-checklist`,
//! the panel). Runtime state only, like the other artifacts: it is not
//! persisted with the agent.

use std::fmt;
use std::str::FromStr;

/// What an item asks of the user. A closed vocabulary, spelled in the tool's
/// arguments as `check` and `question`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecklistItemKind {
    /// Ticked or not.
    Check,
    /// Answered in free text.
    Question,
}

impl fmt::Display for ChecklistItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
                        Self::Check => "check",
                        Self::Question => "question",
                    })
    }
}

impl FromStr for ChecklistItemKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "check" => Ok(Self::Check),
            "question" => Ok(Self::Question),
            other => {
                Err(format!("unknown checklist item kind `{other}`; expected `check` or `question`"))
            }
        }
    }
}

/// One item: what the agent asked, and what the user has said so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistItem {
    /// The agent's name for the item, unique within its checklist. What
    /// carries the user's answer across a republish.
    pub id:      String,
    pub text:    String,
    pub kind:    ChecklistItemKind,
    /// Whether a `Check` is ticked. Always false for a `Question`.
    pub checked: bool,
    /// A `Question`'s answer. Always empty for a `Check`.
    pub answer:  String,
}

impl ChecklistItem {
    /// A fresh item, nothing ticked and nothing answered.
    pub fn new(id: impl Into<String>, text: impl Into<String>, kind: ChecklistItemKind) -> Self {
        Self { id: id.into(),
               text: text.into(),
               kind,
               checked: false,
               answer: String::new() }
    }
}

/// An agent's checklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checklist {
    pub title:     Option<String>,
    pub items:     Vec<ChecklistItem>,
    /// Whether the user has sent the answers to the agent since the
    /// checklist last changed under them.
    pub submitted: bool,
}

impl Checklist {
    /// `items` under `title`, carrying over what the user already entered
    /// in `previous` for every item whose id and kind are unchanged.
    ///
    /// So an agent can refine a checklist - reword an item, add one, drop
    /// one - without wiping the answers the user has given. An item whose
    /// kind changed starts over: a tick does not mean an answer.
    pub fn republished(title: Option<String>, items: Vec<ChecklistItem>,
                       previous: Option<&Checklist>)
                       -> Self {
        let items = items.into_iter()
                         .map(|mut item| {
                             if let Some(earlier) =
                                 previous.and_then(|previous| previous.item(&item.id))
                                         .filter(|earlier| earlier.kind == item.kind)
                             {
                                 item.checked = earlier.checked;
                                 item.answer = earlier.answer.clone();
                             }
                             item
                         })
                         .collect();
        Self { title,
               items,
               submitted: false }
    }

    /// The item named `id`.
    pub fn item(&self, id: &str) -> Option<&ChecklistItem> {
        self.items.iter().find(|item| item.id == id)
    }

    fn item_mut(&mut self, id: &str) -> Option<&mut ChecklistItem> {
        self.items.iter_mut().find(|item| item.id == id)
    }

    /// Ticks or unticks the `Check` named `id`. Returns whether anything
    /// changed; a question, or an id that is not there, is left alone.
    pub fn set_checked(&mut self, id: &str, checked: bool) -> bool {
        let Some(item) = self.item_mut(id)
                             .filter(|item| item.kind == ChecklistItemKind::Check)
        else {
            return false;
        };
        let changed = item.checked != checked;
        item.checked = checked;
        if changed {
            self.submitted = false;
        }
        changed
    }

    /// Sets the answer to the `Question` named `id`. Returns whether
    /// anything changed; a check, or an id that is not there, is left alone.
    pub fn set_answer(&mut self, id: &str, answer: &str) -> bool {
        let Some(item) = self.item_mut(id)
                             .filter(|item| item.kind == ChecklistItemKind::Question)
        else {
            return false;
        };
        let changed = item.answer != answer;
        if changed {
            item.answer = answer.to_owned();
            self.submitted = false;
        }
        changed
    }

    /// The checklist as the agent receives it when the user sends it:
    /// plain text, one line per item, answers indented under their
    /// questions. Not localized - it is addressed to the agent.
    pub fn summary(&self) -> String {
        let mut text = match &self.title {
            Some(title) => format!("Checklist \"{title}\" - my answers:\n"),
            None => "Checklist - my answers:\n".to_owned(),
        };
        for item in &self.items {
            match item.kind {
                ChecklistItemKind::Check => {
                    let mark = if item.checked { "x" } else { " " };
                    text.push_str(&format!("- [{mark}] {} (id: {})\n", item.text, item.id));
                }
                ChecklistItemKind::Question => {
                    let answer = item.answer.trim();
                    let answer = if answer.is_empty() {
                        "(no answer)"
                    }
                    else {
                        answer
                    };
                    text.push_str(&format!("- Q: {} (id: {})\n  A: {}\n",
                                           item.text,
                                           item.id,
                                           answer.replace('\n', "\n     ")));
                }
            }
        }
        text.push_str("\nThe current state is also available from the get-checklist tool.");
        text
    }
}

impl Checklist {
    /// [`Self::summary`] on one line, its lines joined with ` | `: for an
    /// agent driven through a terminal, where whether a newline typed at the
    /// prompt submits early depends on the CLI.
    pub fn summary_line(&self) -> String {
        self.summary()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

#[cfg(test)]
mod tests;

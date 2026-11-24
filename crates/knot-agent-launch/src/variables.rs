//! Prompt variables: the closed set of `{{name}}` placeholders a library
//! prompt or a custom startup prompt may use, the user inputs
//! (`crate::user_inputs`) that share their braces, and the one walker that
//! both expands them and reports the names it does not know.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "Prompt variables" and
//! "Unknown variables and escaping".
//!
//! [`expand`] is pure over a [`PromptContext`] whose values are already
//! resolved. Reading the branch runs `git`, so building the context is the
//! caller's job - see [`crate::startup`] - and each caller knows which thread
//! it is on. Keeping the scan pure is what keeps that read off the render
//! path.

use std::fmt;
use std::str::FromStr;

use crate::user_inputs::DEFAULT_SEPARATOR;
use crate::user_inputs::InputValues;
use crate::user_inputs::KEYWORD;
use crate::user_inputs::user_inputs;

/// Opens a variable.
const OPEN: &str = "{{";
/// Closes a variable.
const CLOSE: &str = "}}";
/// Written before [`OPEN`] to mean a literal `{{`.
const ESCAPE: char = '\\';

/// One of the names that expand. Closed: anything else between the braces is
/// left exactly as typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptVariable {
    AgentName,
    AgentId,
    AgentType,
    Folder,
    FolderName,
    Workspace,
    Branch,
    Date,
    Clipboard,
}

impl PromptVariable {
    /// Every variable, in the order the editors list them.
    pub const ALL: [PromptVariable; 9] = [PromptVariable::AgentName,
                                          PromptVariable::AgentId,
                                          PromptVariable::AgentType,
                                          PromptVariable::Folder,
                                          PromptVariable::FolderName,
                                          PromptVariable::Workspace,
                                          PromptVariable::Branch,
                                          PromptVariable::Date,
                                          PromptVariable::Clipboard];

    /// The name as written between the braces.
    pub fn name(self) -> &'static str {
        match self {
            Self::AgentName => "agent.name",
            Self::AgentId => "agent.id",
            Self::AgentType => "agent.type",
            Self::Folder => "folder",
            Self::FolderName => "folder.name",
            Self::Workspace => "workspace",
            Self::Branch => "branch",
            Self::Date => "date",
            Self::Clipboard => "clipboard",
        }
    }

    /// The variable written the way a prompt uses it, `{{name}}`.
    pub fn placeholder(self) -> String {
        format!("{OPEN}{}{CLOSE}", self.name())
    }
}

impl fmt::Display for PromptVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A name between the braces that is not a [`PromptVariable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVariable(pub String);

impl FromStr for PromptVariable {
    type Err = UnknownVariable;

    /// Case-sensitive: `Folder` is not `folder`.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter()
                 .find(|variable| variable.name() == name)
                 .ok_or_else(|| UnknownVariable(name.to_string()))
    }
}

/// Every value a prompt can be expanded with, already resolved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptContext {
    pub agent_name: String,
    pub agent_id:   String,
    pub agent_type: String,
    /// Absolute path of the agent's folder.
    pub folder:     String,
    /// Empty when the agent belongs to no workspace.
    pub workspace:  String,
    /// Empty when the folder is not in a git repository.
    pub branch:     String,
    /// `YYYY-MM-DD`, local time.
    pub date:       String,
    /// The clipboard's text when the prompt was chosen; empty when it held
    /// none, and always empty for a startup prompt.
    pub clipboard:  String,
    /// What the user typed into the inputs sheet. Empty for a startup
    /// prompt, so every input there takes its default or nothing.
    pub inputs:     InputValues,
}

impl PromptContext {
    fn value(&self, variable: PromptVariable) -> String {
        match variable {
            PromptVariable::AgentName => self.agent_name.clone(),
            PromptVariable::AgentId => self.agent_id.clone(),
            PromptVariable::AgentType => self.agent_type.clone(),
            PromptVariable::Folder => self.folder.clone(),
            PromptVariable::FolderName => knot_core::folder_name(&self.folder).unwrap_or_default(),
            PromptVariable::Workspace => self.workspace.clone(),
            PromptVariable::Branch => self.branch.clone(),
            PromptVariable::Date => self.date.clone(),
            PromptVariable::Clipboard => self.clipboard.clone(),
        }
    }
}

/// `text` with every known variable and every input replaced by its value.
///
/// A single left-to-right pass: values are appended to the output and never
/// rescanned, so a value holding `{{date}}` stays literal by construction.
/// An input takes the typed value, else its label's default - the first
/// non-empty one in the text, whichever occurrence gives it - else nothing.
pub fn expand(text: &str, context: &PromptContext) -> String {
    let defaults = user_inputs(text);
    let mut out = String::with_capacity(text.len());
    walk(text, |piece| match piece {
        Piece::Text(text) => out.push_str(text),
        Piece::Braced { inner: Inner::Variable(variable),
                        .. } => out.push_str(&context.value(variable)),
        Piece::Braced { inner: Inner::Input { label, .. },
                        .. } => {
            let value =
                context.inputs.get(label).or_else(|| {
                                             defaults.iter()
                                                     .find(|input| input.label == label)
                                                     .and_then(|input| input.default.as_deref())
                                         });
            out.push_str(value.unwrap_or_default());
        }
        Piece::Braced { whole, .. } => out.push_str(whole),
    });
    out
}

/// The names in `text` that look like variables but are not one, each once,
/// in the order they first appear - what the editors warn about. A
/// well-formed input is not unknown, and a blank-label one is reported by
/// [`crate::malformed_inputs`] instead.
pub fn unknown_variables(text: &str) -> Vec<String> {
    let mut unknown: Vec<String> = Vec::new();
    walk_inner(text, |inner| {
        if let Inner::Unknown(name) = inner
           && !unknown.iter().any(|seen| seen == name)
        {
            unknown.push(name.to_string());
        }
    });
    unknown
}

/// Whether `text` uses `variable`, escaped occurrences aside - what the
/// startup-prompt editor checks to warn that `{{clipboard}}` is empty there.
pub fn uses_variable(text: &str, variable: PromptVariable) -> bool {
    let mut used = false;
    walk_inner(text, |inner| {
        if matches!(inner, Inner::Variable(found) if found == variable) {
            used = true;
        }
    });
    used
}

/// What the text between a pair of unescaped braces is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Inner<'a> {
    Variable(PromptVariable),
    /// `user_input:` with a non-blank label; `default` is `None` when absent
    /// or empty.
    Input {
        label:   &'a str,
        default: Option<&'a str>,
    },
    /// `user_input:` with a blank label: left as typed.
    MalformedInput,
    /// Anything else: left as typed.
    Unknown(&'a str),
}

/// Classify the text between the braces. The one place a name is decided,
/// so expansion and the editors' warnings cannot disagree.
fn parse_inner(raw: &str) -> Inner<'_> {
    let name = raw.trim();
    if let Some(rest) = name.strip_prefix(KEYWORD) {
        let (label, default) = match rest.split_once(DEFAULT_SEPARATOR) {
            Some((label, default)) => {
                (label.trim(), Some(default.trim()).filter(|d| !d.is_empty()))
            }
            None => (rest.trim(), None),
        };
        if label.is_empty() {
            return Inner::MalformedInput;
        }
        return Inner::Input { label, default };
    }
    match name.parse::<PromptVariable>() {
        Ok(variable) => Inner::Variable(variable),
        Err(_) => Inner::Unknown(name),
    }
}

enum Piece<'a> {
    /// Literal text, written as is.
    Text(&'a str),
    /// A closed pair of braces: `whole` is how it was typed, braces and all.
    Braced { whole: &'a str, inner: Inner<'a> },
}

/// Walk `text`, handing each literal run and each closed pair of unescaped
/// braces to `emit`. The only walker: escaping (`\{{`) and unclosed braces
/// are decided here and nowhere else.
fn walk<'a>(text: &'a str, mut emit: impl FnMut(Piece<'a>)) {
    let mut rest = text;
    while let Some(open) = rest.find(OPEN) {
        let (before, from_open) = rest.split_at(open);
        if before.ends_with(ESCAPE) {
            emit(Piece::Text(&before[..before.len() - ESCAPE.len_utf8()]));
            emit(Piece::Text(OPEN));
            rest = &from_open[OPEN.len()..];
            continue;
        }
        emit(Piece::Text(before));
        let inner = &from_open[OPEN.len()..];
        let Some(close) = inner.find(CLOSE)
        else {
            // Unclosed: the rest is literal.
            emit(Piece::Text(from_open));
            return;
        };
        let whole = &from_open[..OPEN.len() + close + CLOSE.len()];
        emit(Piece::Braced { whole,
                             inner: parse_inner(&inner[..close]) });
        rest = &from_open[whole.len()..];
    }
    emit(Piece::Text(rest));
}

/// [`walk`], keeping only what sits between the braces.
pub(crate) fn walk_inner<'a>(text: &'a str, mut found: impl FnMut(Inner<'a>)) {
    walk(text, |piece| {
        if let Piece::Braced { inner, .. } = piece {
            found(inner);
        }
    });
}

#[cfg(test)]
mod tests;

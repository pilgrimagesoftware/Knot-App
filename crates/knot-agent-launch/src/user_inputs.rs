//! User inputs: `{{user_input:Label}}` and `{{user_input:Label|default}}`,
//! values the user types when a library prompt is inserted into a composer.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "User inputs", "At
//! most five inputs are asked for" and "Startup prompts never ask for
//! inputs".
//!
//! The grammar is parsed by [`crate::variables`]' one walker, so inputs
//! share the variables' braces, escaping and unclosed-brace rules. This
//! module owns what an input is and the values that fill one.

use std::collections::BTreeMap;

pub use crate::consts::MAX_PROMPT_INPUTS;
use crate::variables::Inner;
use crate::variables::walk_inner;

/// Written after the opening braces to make an input.
pub(crate) const KEYWORD: &str = "user_input:";
/// Separates an input's label from its default.
pub(crate) const DEFAULT_SEPARATOR: char = '|';

/// One distinct input a prompt asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInput {
    /// Trimmed and non-empty; what the sheet labels the field with.
    pub label:   String,
    /// The first non-empty default any occurrence of the label gives.
    pub default: Option<String>,
}

/// The values the user typed, keyed by label. A label missing from here
/// expands to its default, then to nothing - which is how a startup prompt,
/// with no values at all, gets the unattended behaviour.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputValues(BTreeMap<String, String>);

impl InputValues {
    /// Set `label`'s value, replacing any earlier one.
    pub fn insert(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.0.insert(label.into(), value.into());
    }

    /// The value typed for `label`, if the user was asked for it.
    pub fn get(&self, label: &str) -> Option<&str> {
        self.0.get(label).map(String::as_str)
    }
}

impl<L: Into<String>, V: Into<String>> FromIterator<(L, V)> for InputValues {
    fn from_iter<I: IntoIterator<Item = (L, V)>>(iter: I) -> Self {
        Self(iter.into_iter()
                 .map(|(label, value)| (label.into(), value.into()))
                 .collect())
    }
}

/// The distinct inputs in `text`, in the order each label first appears,
/// each with the first non-empty default any of its occurrences gives.
pub fn user_inputs(text: &str) -> Vec<UserInput> {
    let mut inputs: Vec<UserInput> = Vec::new();
    walk_inner(text, |inner| {
        let Inner::Input { label, default } = inner
        else {
            return;
        };
        match inputs.iter_mut().find(|input| input.label == label) {
            Some(seen) => {
                if seen.default.is_none() {
                    seen.default = default.map(str::to_string);
                }
            }
            None => inputs.push(UserInput { label:   label.to_string(),
                                            default: default.map(str::to_string), }),
        }
    });
    inputs
}

/// Whether `text` holds an input with a blank label - what the Prompts
/// editor warns about as malformed.
pub fn malformed_inputs(text: &str) -> bool {
    let mut malformed = false;
    walk_inner(text, |inner| {
        if matches!(inner, Inner::MalformedInput) {
            malformed = true;
        }
    });
    malformed
}

#[cfg(test)]
mod tests;

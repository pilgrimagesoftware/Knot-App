//! The typed form a caller renders, and the per-field checks that decide
//! whether it can be submitted.

use serde_json::Value;

use super::AnswerValue;

/// A pending form elicitation, as surfaced to the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionRequest {
    /// The JSON-RPC id the reply goes back under.
    pub rpc_id:       Value,
    /// The session the form belongs to.
    pub session_id:   String,
    /// The tool call that asked, when the form came from one
    /// (AskUserQuestion carries it; an MCP server's form does not).
    pub tool_call_id: Option<String>,
    /// The form's own message, shown above its fields.
    pub message:      String,
    /// The fields, in the order the agent listed them.
    pub fields:       Vec<FormField>,
}

impl QuestionRequest {
    /// The text that best names what is being asked: the first field's
    /// description (AskUserQuestion puts the question there), else its
    /// title, else the form's message.
    pub fn headline(&self) -> &str {
        self.fields
            .first()
            .and_then(|field| field.description.as_deref().or(field.title.as_deref()))
            .unwrap_or(&self.message)
    }
}

/// One property of the form's schema.
#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    /// The property name; the answer is sent under it.
    pub key:         String,
    pub title:       Option<String>,
    pub description: Option<String>,
    /// Listed in the schema's object-level `required`.
    pub required:    bool,
    pub kind:        FieldKind,
}

/// What a field holds, with the schema's default and bounds for it.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    /// Exactly one of `choices` (`oneOf` of consts, or `enum`).
    Single {
        choices: Vec<Choice>,
        default: Option<String>,
    },
    /// Any number of `choices` (`type: array` whose items are a choice list).
    Multi {
        choices:   Vec<Choice>,
        default:   Vec<String>,
        min_items: Option<u64>,
        max_items: Option<u64>,
    },
    /// Free text. `format` (email, uri, date) is not validated here: the
    /// requester validates what it receives.
    Text {
        default:    Option<String>,
        min_length: Option<u64>,
        max_length: Option<u64>,
    },
    Number {
        integer: bool,
        default: Option<f64>,
        minimum: Option<f64>,
        maximum: Option<f64>,
    },
    Boolean {
        default: Option<bool>,
    },
}

/// One option of a choice field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// What is sent back when it is picked.
    pub value:       String,
    /// What is shown; the value when the schema gives no title.
    pub title:       String,
    pub description: Option<String>,
}

impl FormField {
    /// Whether `value` (`None` for an empty field) may be submitted for this
    /// field: present when required, of the field's type, one of its choices,
    /// and inside its bounds.
    pub fn accepts(&self, value: Option<&AnswerValue>) -> bool {
        let Some(value) = value
        else {
            return !self.required;
        };
        match &self.kind {
            FieldKind::Single { choices, .. } => {
                // A single choice's free text arrives on its own `_custom`
                // field, so a value here must be one of the choices.
                let AnswerValue::Text(text) = value
                else {
                    return false;
                };
                choices.iter().any(|choice| choice.value == *text)
            }
            FieldKind::Multi { choices,
                               min_items,
                               max_items,
                               .. } => {
                let AnswerValue::Choices(picked) = value
                else {
                    return false;
                };
                let count = picked.len() as u64;
                picked.iter()
                      .all(|value| choices.iter().any(|choice| choice.value == *value))
                && min_items.is_none_or(|min| count >= min)
                && max_items.is_none_or(|max| count <= max)
                && (!self.required || count > 0)
            }
            FieldKind::Text { min_length,
                              max_length,
                              .. } => {
                let AnswerValue::Text(text) = value
                else {
                    return false;
                };
                let length = text.chars().count() as u64;
                min_length.is_none_or(|min| length >= min)
                && max_length.is_none_or(|max| length <= max)
            }
            FieldKind::Number { integer,
                                minimum,
                                maximum,
                                .. } => {
                let Some(number) = value.as_number().and_then(serde_json::Number::as_f64)
                else {
                    return false;
                };
                (!integer || number.fract() == 0.0)
                && minimum.is_none_or(|min| number >= min)
                && maximum.is_none_or(|max| number <= max)
            }
            FieldKind::Boolean { .. } => matches!(value, AnswerValue::Boolean(_)),
        }
    }
}

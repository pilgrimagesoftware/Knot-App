//! The caller's reply to a form, and its `elicitation/create` result.

use serde_json::{Map, Number, Value, json};

/// How the user answered a form.
#[derive(Debug, Clone, PartialEq)]
pub enum FormAnswer {
    /// Submit: `accept` with these fields. A field left empty is absent.
    Submitted(Vec<(String, AnswerValue)>),
    /// Skip: `decline` - `claude-agent-acp` tells the model the user skipped.
    Skipped,
    /// Stop or the session ending: `cancel` - the adapter aborts the tool.
    Cancelled,
}

/// One filled field, typed as its schema declares.
#[derive(Debug, Clone, PartialEq)]
pub enum AnswerValue {
    /// A single choice's value, or free text.
    Text(String),
    /// A multiple choice's picked values.
    Choices(Vec<String>),
    Number(Number),
    Boolean(bool),
}

impl AnswerValue {
    /// The number, when this is one.
    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Self::Number(number) => Some(number),
            Self::Text(_) | Self::Choices(_) | Self::Boolean(_) => None,
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Self::Text(text) => Value::String(text.clone()),
            Self::Choices(picked) => {
                Value::Array(picked.iter().cloned().map(Value::String).collect())
            }
            Self::Number(number) => Value::Number(number.clone()),
            Self::Boolean(flag) => Value::Bool(*flag),
        }
    }
}

impl FormAnswer {
    /// The `elicitation/create` result: `{"action":"accept","content":{..}}`,
    /// `{"action":"decline"}` or `{"action":"cancel"}` (ACP's
    /// `CreateElicitationResponse`).
    pub(crate) fn to_result(&self) -> Value {
        match self {
            Self::Submitted(fields) => {
                let content: Map<String, Value> =
                    fields.iter()
                          .map(|(key, value)| (key.clone(), value.to_value()))
                          .collect();
                json!({ "action": "accept", "content": content })
            }
            Self::Skipped => json!({ "action": "decline" }),
            Self::Cancelled => json!({ "action": "cancel" }),
        }
    }
}

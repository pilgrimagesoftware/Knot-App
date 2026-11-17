//! `elicitation/create` params to a [`QuestionRequest`], or the reason the
//! form cannot be shown.
//!
//! Field order is the wire order because `knot-acp` builds `serde_json`
//! with `preserve_order`: the transport decodes each line to a `Value`, so
//! the order is fixed there and nothing here could recover it.

use std::fmt;

use serde_json::{Map, Value};

use super::{Choice, FieldKind, FormField, QuestionRequest};

/// Why a form was refused (answered `-32602`, never surfaced).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// `mode` is `url` or something else Knot did not advertise.
    UnsupportedMode(String),
    /// No `sessionId`.
    MissingSession,
    /// `requestedSchema` is not an object schema with `properties`.
    MalformedSchema,
    /// A property is an object: the subset is flat.
    NestedObject(String),
    /// An array whose items are not a list of choices.
    ArrayWithoutChoices(String),
    /// A property with neither a known `type` nor choices.
    UnknownType(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMode(mode) => write!(f, "elicitation mode {mode:?} is not supported"),
            Self::MissingSession => f.write_str("elicitation has no sessionId"),
            Self::MalformedSchema => f.write_str("requestedSchema is not an object schema"),
            Self::NestedObject(key) => write!(f, "field {key:?} is an object"),
            Self::ArrayWithoutChoices(key) => {
                write!(f, "field {key:?} is an array without choices")
            }
            Self::UnknownType(key) => write!(f, "field {key:?} has no supported type"),
        }
    }
}

/// Parses a form-mode `elicitation/create` request's `params`. `mode` may be
/// absent (MCP's form requests carry none); only `form` is accepted.
pub(crate) fn parse_form(rpc_id: Value, params: &Value) -> Result<QuestionRequest, Refusal> {
    if let Some(mode) = params.get("mode").and_then(Value::as_str)
       && mode != "form"
    {
        return Err(Refusal::UnsupportedMode(mode.to_owned()));
    }
    let session_id = params.get("sessionId")
                           .and_then(Value::as_str)
                           .ok_or(Refusal::MissingSession)?
                           .to_owned();
    let schema = params.get("requestedSchema")
                       .ok_or(Refusal::MalformedSchema)?;
    let no_properties = Map::new();
    let properties = match schema.get("properties") {
        None => &no_properties,
        Some(Value::Object(properties)) => properties,
        Some(_) => return Err(Refusal::MalformedSchema),
    };
    let required: Vec<&str> = schema.get("required")
                                    .and_then(Value::as_array)
                                    .map(|keys| keys.iter().filter_map(Value::as_str).collect())
                                    .unwrap_or_default();
    let fields = properties.iter()
                           .map(|(key, property)| {
                               Ok(FormField { key:         key.clone(),
                                              title:       text(property, "title"),
                                              description: text(property, "description"),
                                              required:    required.contains(&key.as_str()),
                                              kind:        field_kind(key, property)?, })
                           })
                           .collect::<Result<_, Refusal>>()?;
    Ok(QuestionRequest { rpc_id,
                         session_id,
                         tool_call_id: text(params, "toolCallId"),
                         message: text(params, "message").unwrap_or_default(),
                         fields })
}

fn field_kind(key: &str, property: &Value) -> Result<FieldKind, Refusal> {
    if declares_choices(property) {
        let choices = choices(property).ok_or_else(|| Refusal::UnknownType(key.to_owned()))?;
        return Ok(FieldKind::Single { choices,
                                      default: text(property, "default") });
    }
    match property.get("type").and_then(Value::as_str) {
        Some("array") => {
            let choices = property.get("items")
                                  .and_then(choices)
                                  .ok_or_else(|| Refusal::ArrayWithoutChoices(key.to_owned()))?;
            let default = property.get("default")
                                  .and_then(Value::as_array)
                                  .map(|values| {
                                      values.iter()
                                            .filter_map(Value::as_str)
                                            .map(str::to_owned)
                                            .collect()
                                  })
                                  .unwrap_or_default();
            Ok(FieldKind::Multi { choices,
                                  default,
                                  min_items: unsigned(property, "minItems"),
                                  max_items: unsigned(property, "maxItems") })
        }
        Some("string") => Ok(FieldKind::Text { default:    text(property, "default"),
                                               min_length: unsigned(property, "minLength"),
                                               max_length: unsigned(property, "maxLength"), }),
        Some(kind @ ("number" | "integer")) => {
            Ok(FieldKind::Number { integer: kind == "integer",
                                   default: float(property, "default"),
                                   minimum: float(property, "minimum"),
                                   maximum: float(property, "maximum"), })
        }
        Some("boolean") => {
            Ok(FieldKind::Boolean { default: property.get("default").and_then(Value::as_bool), })
        }
        Some("object") => Err(Refusal::NestedObject(key.to_owned())),
        Some(_) | None => Err(Refusal::UnknownType(key.to_owned())),
    }
}

/// Whether `schema` declares a choice list at all, well-formed or not.
fn declares_choices(schema: &Value) -> bool {
    ["oneOf", "anyOf", "enum"].iter()
                              .any(|key| schema.get(key).is_some())
}

/// The choice list `schema` declares, if it is one: `oneOf`/`anyOf` of
/// `{const, title?, description?}`, or `enum` with optional `enumNames`.
/// `None` when it declares none, or when any entry is not a string const.
fn choices(schema: &Value) -> Option<Vec<Choice>> {
    if let Some(entries) = schema.get("oneOf")
                                 .or_else(|| schema.get("anyOf"))
                                 .and_then(Value::as_array)
    {
        return entries.iter()
                      .map(|entry| {
                          let value = entry.get("const")?.as_str()?.to_owned();
                          Some(Choice { title: text(entry, "title").unwrap_or_else(|| {
                                                                       value.clone()
                                                                   }),
                                        description: text(entry, "description"),
                                        value })
                      })
                      .collect();
    }
    let values = schema.get("enum")?.as_array()?;
    let names = schema.get("enumNames").and_then(Value::as_array);
    values.iter()
          .enumerate()
          .map(|(index, value)| {
              let value = value.as_str()?.to_owned();
              let title = names.and_then(|names| names.get(index))
                               .and_then(Value::as_str)
                               .map_or_else(|| value.clone(), str::to_owned);
              Some(Choice { value,
                            title,
                            description: None })
          })
          .collect()
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn unsigned(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn float(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

//! Unit tests for [`super`].
//!
//! `ask_user_question_params` is the request `claude-agent-acp` 0.107.3 sent
//! a live client for an AskUserQuestion call (design.md, "Wire path"), so the
//! parser is checked against the adapter's real shape rather than one
//! inferred from these types.

use serde_json::{Value, json};

use super::*;

fn ask_user_question_params() -> Value {
    json!({
        "mode": "form",
        "sessionId": "9ab61743-681d-4c93-a8c3-ba1488b4ee61",
        "toolCallId": "toolu_019aCzvwBEKnazTtpwuzeU18",
        "message": "Please answer the following questions.",
        "requestedSchema": { "type": "object", "properties": {
            "question_0": { "type": "string", "title": "Color", "description": "Which color?",
                "oneOf": [
                    { "const": "Red", "title": "Red", "description": "Pick red" },
                    { "const": "Green", "title": "Green", "description": "Pick green" },
                    { "const": "Blue", "title": "Blue", "description": "Pick blue" } ] },
            "question_0_custom": { "type": "string", "title": "Other",
                "description": "Type your own answer, or add a note to the option you chose above (optional)." },
            "question_1": { "type": "array", "title": "Fruits", "description": "Which fruits?",
                "items": { "anyOf": [
                    { "const": "Apple", "title": "Apple", "description": "Include apple" },
                    { "const": "Pear", "title": "Pear", "description": "Include pear" },
                    { "const": "Plum", "title": "Plum", "description": "Include plum" } ] } },
            "question_1_custom": { "type": "string", "title": "Other",
                "description": "Type your own answer to add to your selection above (optional)." }
        } }
    })
}

fn parse(params: &Value) -> Result<QuestionRequest, Refusal> {
    parse_form(json!(7), params)
}

fn keys(request: &QuestionRequest) -> Vec<&str> {
    request.fields
           .iter()
           .map(|field| field.key.as_str())
           .collect()
}

#[test]
fn the_adapters_ask_user_question_form_parses_in_order() {
    let request = parse(&ask_user_question_params()).expect("parses");
    assert_eq!(request.rpc_id, json!(7));
    assert_eq!(request.session_id, "9ab61743-681d-4c93-a8c3-ba1488b4ee61");
    assert_eq!(request.tool_call_id.as_deref(),
               Some("toolu_019aCzvwBEKnazTtpwuzeU18"));
    assert_eq!(keys(&request),
               ["question_0",
                "question_0_custom",
                "question_1",
                "question_1_custom"]);
    assert_eq!(request.headline(), "Which color?");

    let FieldKind::Single { choices, .. } = &request.fields[0].kind
    else {
        panic!("question_0 is a single choice: {:?}",
               request.fields[0].kind);
    };
    assert_eq!(choices[1],
               Choice { value:       "Green".to_owned(),
                        title:       "Green".to_owned(),
                        description: Some("Pick green".to_owned()), });
    assert!(matches!(request.fields[1].kind, FieldKind::Text { .. }));
    assert!(matches!(&request.fields[2].kind, FieldKind::Multi { choices, .. } if choices.len() == 3));
}

/// `preserve_order` is what keeps this: without it the keys come back
/// sorted and `question_10` lands before `question_2`. Runs under
/// `cargo test -p knot-acp`, where gpui's unification does not apply.
#[test]
fn field_order_survives_more_than_nine_fields() {
    let properties: serde_json::Map<String, Value> =
        (0..12).map(|index| (format!("question_{index}"), json!({ "type": "string" })))
               .collect();
    let line = json!({ "sessionId": "s", "requestedSchema": { "type": "object", "properties": properties } })
        .to_string();
    let params: Value = serde_json::from_str(&line).expect("decodes");
    let request = parse(&params).expect("parses");
    let expected: Vec<String> = (0..12).map(|index| format!("question_{index}")).collect();
    assert_eq!(keys(&request), expected);
}

#[test]
fn an_mcp_form_parses_every_field_type() {
    let params = json!({ "sessionId": "s", "message": "Configure the server",
        "requestedSchema": { "type": "object", "required": ["name", "port"], "properties": {
            "name": { "type": "string", "minLength": 2, "maxLength": 8, "default": "knot" },
            "port": { "type": "integer", "minimum": 1, "maximum": 65535, "default": 8080 },
            "ratio": { "type": "number" },
            "verbose": { "type": "boolean", "default": true },
            "level": { "type": "string", "enum": ["lo", "hi"], "enumNames": ["Low", "High"], "default": "hi" },
            "tags": { "type": "array", "minItems": 1, "maxItems": 2,
                      "items": { "enum": ["a", "b", "c"] }, "default": ["a"] },
            "email": { "type": "string", "format": "email" }
        } } });
    let request = parse(&params).expect("parses");
    assert_eq!(request.tool_call_id, None);
    assert_eq!(keys(&request),
               ["name", "port", "ratio", "verbose", "level", "tags", "email"]);
    let required: Vec<bool> = request.fields.iter().map(|field| field.required).collect();
    assert_eq!(required, [true, true, false, false, false, false, false]);
    assert_eq!(request.fields[0].kind,
               FieldKind::Text { default:    Some("knot".to_owned()),
                                 min_length: Some(2),
                                 max_length: Some(8), });
    assert_eq!(request.fields[1].kind,
               FieldKind::Number { integer: true,
                                   default: Some(8080.0),
                                   minimum: Some(1.0),
                                   maximum: Some(65535.0), });
    assert!(matches!(request.fields[2].kind,
                     FieldKind::Number { integer: false, .. }));
    assert_eq!(request.fields[3].kind,
               FieldKind::Boolean { default: Some(true), });
    let FieldKind::Single { choices, default } = &request.fields[4].kind
    else {
        panic!("level is a single choice");
    };
    assert_eq!((choices[1].value.as_str(), choices[1].title.as_str()),
               ("hi", "High"));
    assert_eq!(default.as_deref(), Some("hi"));
    assert_eq!(request.fields[5].kind,
               FieldKind::Multi { choices:   ["a", "b", "c"].map(|value| {
                                                                Choice { value:
                                                                             value.to_owned(),
                                                                         title:
                                                                             value.to_owned(),
                                                                         description: None, }
                                                            })
                                                            .to_vec(),
                                  default:   vec!["a".to_owned()],
                                  min_items: Some(1),
                                  max_items: Some(2), });
    assert!(matches!(request.fields[6].kind, FieldKind::Text { .. }));
    assert_eq!(request.headline(), "Configure the server");
}

#[test]
fn unrenderable_forms_are_refused() {
    let form = |properties: Value| {
        json!({ "sessionId": "s",
        "requestedSchema": { "type": "object", "properties": properties } })
    };
    assert_eq!(parse(&json!({ "mode": "url", "sessionId": "s", "url": "https://x" })),
               Err(Refusal::UnsupportedMode("url".to_owned())));
    assert_eq!(parse(&form(json!({ "where": { "type": "object" } }))),
               Err(Refusal::NestedObject("where".to_owned())));
    assert_eq!(parse(&form(json!({ "list": { "type": "array", "items": { "type": "string" } } }))),
               Err(Refusal::ArrayWithoutChoices("list".to_owned())));
    assert_eq!(parse(&form(json!({ "odd": { "type": "null" } }))),
               Err(Refusal::UnknownType("odd".to_owned())));
    assert_eq!(parse(&form(json!({ "bad": { "oneOf": [{ "const": 3 }] } }))),
               Err(Refusal::UnknownType("bad".to_owned())));
    assert_eq!(parse(&json!({ "requestedSchema": { "type": "object", "properties": {} } })),
               Err(Refusal::MissingSession));
}

#[test]
fn a_form_without_mode_or_fields_is_a_plain_confirmation() {
    let request = parse(&json!({ "sessionId": "s", "message": "Proceed?",
                                 "requestedSchema": { "type": "object" } })).expect("parses");
    assert!(request.fields.is_empty());
    assert_eq!(request.headline(), "Proceed?");
}

fn field(kind: FieldKind, required: bool) -> FormField {
    FormField { key: "f".to_owned(),
                title: None,
                description: None,
                required,
                kind }
}

#[test]
fn accepts_enforces_required_choices_and_bounds() {
    let text = field(FieldKind::Text { default:    None,
                                       min_length: Some(2),
                                       max_length: Some(3), },
                     true);
    assert!(!text.accepts(None));
    assert!(!text.accepts(Some(&AnswerValue::Text("a".to_owned()))));
    assert!(text.accepts(Some(&AnswerValue::Text("ab".to_owned()))));
    assert!(!text.accepts(Some(&AnswerValue::Text("abcd".to_owned()))));

    let number = field(FieldKind::Number { integer: true,
                                           default: None,
                                           minimum: Some(1.0),
                                           maximum: Some(10.0), },
                       false);
    assert!(number.accepts(None));
    assert!(number.accepts(Some(&AnswerValue::Number(5.into()))));
    assert!(!number.accepts(Some(&AnswerValue::Number(11.into()))));
    let half = serde_json::Number::from_f64(2.5).expect("finite");
    assert!(!number.accepts(Some(&AnswerValue::Number(half))));
    assert!(!number.accepts(Some(&AnswerValue::Text("5".to_owned()))));

    let choice = |value: &str| Choice { value:       value.to_owned(),
                                        title:       value.to_owned(),
                                        description: None, };
    let multi = field(FieldKind::Multi { choices:   vec![choice("a"), choice("b"), choice("c")],
                                         default:   vec![],
                                         min_items: None,
                                         max_items: Some(2), },
                      true);
    let picks =
        |values: &[&str]| AnswerValue::Choices(values.iter().map(|v| (*v).to_owned()).collect());
    assert!(!multi.accepts(Some(&picks(&[]))));
    assert!(multi.accepts(Some(&picks(&["a", "c"]))));
    assert!(!multi.accepts(Some(&picks(&["a", "b", "c"]))));
    assert!(!multi.accepts(Some(&picks(&["z"]))));

    let single = field(FieldKind::Single { choices: vec![choice("a")],
                                           default: None, },
                       false);
    assert!(single.accepts(Some(&AnswerValue::Text("a".to_owned()))));
    assert!(!single.accepts(Some(&AnswerValue::Text("z".to_owned()))));
}

#[test]
fn answers_serialize_as_acp_elicitation_results() {
    let submitted =
        FormAnswer::Submitted(vec![("question_0".to_owned(), AnswerValue::Text("Red".to_owned())),
                                   ("question_1".to_owned(),
                                    AnswerValue::Choices(vec!["Apple".to_owned(),
                                                              "Plum".to_owned()])),
                                   ("port".to_owned(), AnswerValue::Number(8080.into())),
                                   ("verbose".to_owned(), AnswerValue::Boolean(false)),]);
    assert_eq!(submitted.to_result(),
               json!({ "action": "accept", "content": {
                   "question_0": "Red", "question_1": ["Apple", "Plum"],
                   "port": 8080, "verbose": false } }));
    assert_eq!(FormAnswer::Skipped.to_result(),
               json!({ "action": "decline" }));
    assert_eq!(FormAnswer::Cancelled.to_result(),
               json!({ "action": "cancel" }));
}

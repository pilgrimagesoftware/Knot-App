//! The catalog's input schemas, as a function-calling backend reads them.

use super::*;

/// Every array in `schema`, at any depth, as `path` → whether it has `items`.
fn arrays_without_items(path: &str, schema: &serde_json::Value, out: &mut Vec<String>) {
    let Some(object) = schema.as_object()
    else {
        return;
    };
    if object.get("type").and_then(serde_json::Value::as_str) == Some("array")
       && !object.contains_key("items")
    {
        out.push(path.to_owned());
    }
    if let Some(properties) = object.get("properties")
                                    .and_then(serde_json::Value::as_object)
    {
        for (name, property) in properties {
            arrays_without_items(&format!("{path}.{name}"), property, out);
        }
    }
    if let Some(items) = object.get("items") {
        arrays_without_items(&format!("{path}[]"), items, out);
    }
}

/// #431: Gemini (agy, Gemini CLI) rejects a function whose array parameter
/// has no `items`, and fails the whole request over one such tool - every
/// turn of an agy session with Knot's tools attached ended in
/// `INVALID_ARGUMENT ... properties[capabilities].items: missing field`.
/// Walked over the serialized schema, so a property added later without
/// `items` fails here rather than in a user's session.
#[test]
fn every_array_parameter_declares_its_items() {
    let mut missing = Vec::new();
    for tool in tool_catalog() {
        let schema = serde_json::to_value(&tool.input_schema).expect("schemas serialize");
        arrays_without_items(&tool.name, &schema, &mut missing);
    }
    assert!(missing.is_empty(),
            "array parameters without `items`: {missing:?}");
}

/// `plan-tasks`' element schema names the fields its parser reads.
#[test]
fn plan_tasks_describes_a_task() {
    let plan = tool_catalog().into_iter()
                             .find(|tool| tool.name == consts::PLAN_TASKS)
                             .expect("plan-tasks is in the catalog");
    let items = plan.input_schema.properties["tasks"].items
                                                     .clone()
                                                     .expect("tasks has items");
    for field in ["id",
                  "goal",
                  "assignee",
                  "capabilities",
                  "dependsOn",
                  "resetContext"]
    {
        assert!(items["properties"].get(field).is_some(), "{field}");
    }
    assert_eq!(items["required"], serde_json::json!(["id", "goal"]));
}

use knot_agents::CreateOptions;
use serde_json::{Value, json};

use super::*;

fn store_with_agent() -> (AgentStore, uuid::Uuid) {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    (store, id)
}

fn show(store: &mut AgentStore, id: uuid::Uuid, items: Value) -> ToolCallResult {
    show_checklist(store,
                   &json!({"agentId": id.to_string(), "title": "Release", "items": items}))
}

fn read(store: &AgentStore, id: uuid::Uuid) -> Value {
    let result = get_checklist(store, &json!({"agentId": id.to_string()}));
    assert_ne!(result.is_error, Some(true), "{}", result.content[0].text);
    serde_json::from_str(&result.content[0].text).expect("get-checklist answers JSON")
}

#[test]
fn a_shown_checklist_lands_on_the_agent() {
    let (mut store, id) = store_with_agent();

    let result = show(&mut store,
                      id,
                      json!([{"id": "tests", "text": "Tests pass"},
                             {"id": "name", "text": "Name it", "kind": "question"}]));

    assert_ne!(result.is_error, Some(true), "{}", result.content[0].text);
    let checklist = store.agent(id)
                         .unwrap()
                         .checklist
                         .clone()
                         .expect("a checklist was set");
    assert_eq!(checklist.title.as_deref(), Some("Release"));
    assert_eq!(checklist.items.len(), 2);
    assert_eq!(checklist.items[0].kind,
               ChecklistItemKind::Check,
               "check is the default kind");
    assert_eq!(checklist.items[1].kind, ChecklistItemKind::Question);
}

#[test]
fn malformed_items_are_refused_with_the_reason() {
    let (mut store, id) = store_with_agent();
    let cases = [(json!([]), "at least one"),
                 (json!([{"text": "no id"}]), "\"id\""),
                 (json!([{"id": "a"}]), "\"text\""),
                 (json!([{"id": "a", "text": "x", "kind": "todo"}]),
                  "unknown checklist item kind"),
                 (json!([{"id": "a", "text": "x"}, {"id": "a", "text": "y"}]), "used twice")];

    for (items, reason) in cases {
        let result = show(&mut store, id, items.clone());
        assert_eq!(result.is_error, Some(true), "{items} was accepted");
        assert!(result.content[0].text.contains(reason),
                "{items}: {}",
                result.content[0].text);
    }
    assert!(store.agent(id).unwrap().checklist.is_none(),
            "a refused call must not leave a checklist behind");
}

/// What the user did in the panel is what the agent reads back.
#[test]
fn get_checklist_reads_the_users_answers() {
    let (mut store, id) = store_with_agent();
    show(&mut store,
         id,
         json!([{"id": "tests", "text": "Tests pass"},
                {"id": "name", "text": "Name it", "kind": "question"}]));
    store.edit_checklist(id, |list| {
             list.set_checked("tests", true);
             list.set_answer("name", "Knot");
         });

    let state = read(&store, id);

    assert_eq!(state["title"], "Release");
    assert_eq!(state["submitted"], false);
    assert_eq!(state["items"][0]["kind"], "check");
    assert_eq!(state["items"][0]["checked"], true);
    assert!(state["items"][0].get("answer").is_none(),
            "a check has no answer field");
    assert_eq!(state["items"][1]["kind"], "question");
    assert_eq!(state["items"][1]["answer"], "Knot");
}

#[test]
fn get_checklist_without_one_is_an_error_naming_the_way_to_show_one() {
    let (store, id) = store_with_agent();

    let result = get_checklist(&store, &json!({"agentId": id.to_string()}));

    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("show-checklist"));
}

#[test]
fn an_unknown_agent_is_reported() {
    let (mut store, _) = store_with_agent();

    let result = show_checklist(&mut store,
                                &json!({"agentId": "nobody", "items": [{"id": "a", "text": "x"}]}));

    assert_eq!(result.is_error, Some(true));
}

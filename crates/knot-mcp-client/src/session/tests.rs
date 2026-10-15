//! The conversation against a scripted server, so each rule is pinned
//! without a process or a socket.

use std::collections::VecDeque;

use serde_json::{Value, json};

use super::{Exchange, answers, list_tools_over};
use crate::consts::{MAX_PAGES, MAX_TOOLS};
use crate::error::{ClientError, Result};

/// Answers each request with the next scripted result, and records what it
/// was sent.
#[derive(Default)]
struct Scripted {
    results: VecDeque<Value>,
    sent:    Vec<Value>,
}

impl Scripted {
    fn answering(results: impl IntoIterator<Item = Value>) -> Self {
        Self { results: results.into_iter().collect(),
               sent:    Vec::new(), }
    }

    fn methods(&self) -> Vec<&str> {
        self.sent
            .iter()
            .filter_map(|message| message.get("method").and_then(Value::as_str))
            .collect()
    }
}

impl Exchange for Scripted {
    fn request(&mut self, id: u64, message: &Value) -> Result<Value> {
        self.sent.push(message.clone());
        let answer = self.results.pop_front().ok_or(ClientError::Closed)?;
        Ok(json!({ "jsonrpc": "2.0", "id": id, "result": answer }))
    }

    fn notify(&mut self, message: &Value) -> Result<()> {
        self.sent.push(message.clone());
        Ok(())
    }
}

fn tools(names: &[&str]) -> Value {
    json!({ "tools": names.iter()
                          .map(|name| json!({ "name": name, "description": format!("does {name}") }))
                          .collect::<Vec<_>>() })
}

#[test]
fn initializes_then_lists() {
    let mut server = Scripted::answering([json!({}), tools(&["read", "write"])]);

    let listed = list_tools_over(&mut server).expect("listed");

    assert_eq!(server.methods(),
               ["initialize", "notifications/initialized", "tools/list"]);
    assert_eq!(listed.iter()
                     .map(|tool| tool.name.as_str())
                     .collect::<Vec<_>>(),
               ["read", "write"]);
    assert_eq!(listed[0].description.as_deref(), Some("does read"));
}

#[test]
fn a_tool_without_a_description_is_still_listed() {
    let mut server = Scripted::answering([json!({}), json!({ "tools": [{ "name": "bare" }] })]);

    let listed = list_tools_over(&mut server).expect("listed");

    assert_eq!(listed[0].name, "bare");
    assert_eq!(listed[0].description, None);
}

#[test]
fn follows_the_cursor_across_pages() {
    let mut first = tools(&["a"]);
    first["nextCursor"] = json!("page-2");
    let mut server = Scripted::answering([json!({}), first, tools(&["b"])]);

    let listed = list_tools_over(&mut server).expect("listed");

    assert_eq!(listed.len(), 2);
    assert_eq!(server.sent
                     .last()
                     .and_then(|message| message.pointer("/params/cursor")),
               Some(&json!("page-2")));
}

#[test]
fn a_cursor_that_never_ends_stops_at_the_page_bound() {
    let mut endless = tools(&["a"]);
    endless["nextCursor"] = json!("again");
    let pages = std::iter::repeat_n(endless, MAX_PAGES + 5);
    let mut server = Scripted::answering(std::iter::once(json!({})).chain(pages));

    let listed = list_tools_over(&mut server).expect("listed");

    assert_eq!(listed.len(), MAX_PAGES);
}

#[test]
fn a_huge_answer_is_cut_at_the_tool_bound() {
    let names = (0..MAX_TOOLS + 50).map(|index| format!("t{index}"))
                                   .collect::<Vec<_>>();
    let names = names.iter().map(String::as_str).collect::<Vec<_>>();
    let mut server = Scripted::answering([json!({}), tools(&names)]);

    assert_eq!(list_tools_over(&mut server).expect("listed").len(),
               MAX_TOOLS);
}

#[test]
fn an_error_response_is_reported_with_its_code() {
    struct Refusing;
    impl Exchange for Refusing {
        fn request(&mut self, id: u64, _: &Value) -> Result<Value> {
            Ok(json!({ "jsonrpc": "2.0", "id": id,
                       "error": { "code": -32601, "message": "no such method" } }))
        }

        fn notify(&mut self, _: &Value) -> Result<()> {
            Ok(())
        }
    }

    let error = list_tools_over(&mut Refusing).expect_err("refused");

    assert!(matches!(error, ClientError::Rpc { code: -32601, .. }),
            "{error:?}");
}

#[test]
fn only_a_response_with_the_same_id_answers() {
    assert!(answers(&json!({ "id": 2, "result": {} }), 2));
    assert!(answers(&json!({ "id": 2, "error": {} }), 2));
    assert!(!answers(&json!({ "id": 3, "result": {} }), 2));
    assert!(!answers(&json!({ "method": "notifications/progress" }), 2));
    assert!(!answers(&json!({ "id": 2, "method": "roots/list" }), 2),
            "a request of the server's own that reuses the id is not an answer");
}

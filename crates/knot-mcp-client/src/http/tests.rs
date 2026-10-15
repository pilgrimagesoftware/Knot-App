//! Reading an answer out of a response body, without a server.

use std::io::Cursor;

use serde_json::json;

use super::{answer_in_body, answer_in_stream};
use crate::error::ClientError;

#[test]
fn a_plain_body_answers() {
    let answer = answer_in_body(json!({ "id": 1, "result": { "ok": true } }), 1).expect("answer");
    assert_eq!(answer["result"]["ok"], json!(true));
}

#[test]
fn a_batch_body_answers_with_the_matching_message() {
    let body = json!([{ "method": "notifications/message" },
                      { "id": 4, "result": { "tools": [] } }]);
    assert!(answer_in_body(body, 4).is_ok());
}

#[test]
fn a_body_with_another_id_is_not_an_answer() {
    assert!(matches!(answer_in_body(json!({ "id": 9, "result": {} }), 1),
                     Err(ClientError::Protocol(_))));
}

#[test]
fn a_stream_answers_past_the_notifications_before_it() {
    let stream = "event: message\n\
                  data: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\
                  \n\
                  event: message\n\
                  data: {\"jsonrpc\":\"2.0\",\"id\":2,\n\
                  data: \"result\":{\"tools\":[]}}\n\
                  \n";
    let answer = answer_in_stream(Cursor::new(stream), 2).expect("answer");
    assert_eq!(answer["result"], json!({ "tools": [] }));
}

#[test]
fn a_stream_without_the_closing_blank_line_still_answers() {
    let stream = "data: {\"id\":3,\"result\":{}}";
    assert!(answer_in_stream(Cursor::new(stream), 3).is_ok());
}

#[test]
fn a_stream_that_ends_unanswered_is_closed() {
    let stream = "data: {\"method\":\"ping\"}\n\n";
    assert!(matches!(answer_in_stream(Cursor::new(stream), 3),
                     Err(ClientError::Closed)));
}

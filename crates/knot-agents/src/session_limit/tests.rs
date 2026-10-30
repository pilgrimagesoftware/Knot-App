//! Recognising a usage limit from the error that answers a prompt (#318).

use super::SessionLimit;

/// The error from the issue's screenshot, verbatim.
#[test]
fn claudes_session_limit_is_recognised_with_its_reset_time() {
    let error = "The agent could not answer: acp error response (code -32603): Internal \
                 error: You've hit your session limit · resets 5:20pm (America/Los_Angeles)";

    assert_eq!(SessionLimit::from_error(error),
               Some(SessionLimit { resets: Some("5:20pm (America/Los_Angeles)".to_owned()), }));
}

#[test]
fn a_usage_limit_without_a_reset_time_is_still_a_limit() {
    assert_eq!(SessionLimit::from_error("You've hit your usage limit."),
               Some(SessionLimit { resets: None }));
}

#[test]
fn any_other_error_is_not_a_limit() {
    for error in ["quota exhausted",
                  "acp error response (code -32603): Internal error",
                  "connection reset by peer"]
    {
        assert_eq!(SessionLimit::from_error(error), None, "{error}");
    }
}

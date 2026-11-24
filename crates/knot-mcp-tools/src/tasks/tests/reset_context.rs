//! `resetContext`: a task that asks for a fresh conversation flags its
//! assignee on dispatch, never the caller, and waits for the assignee's task
//! in flight.

use super::*;

/// #331: a step that asks for a fresh conversation flags its assignee when
/// it is dispatched, and only then - the plan alone restarts nobody.
#[test]
fn a_reset_context_task_flags_its_assignee_on_dispatch() {
    let mut fx = fixture();
    let worker = fx.worker.to_string();
    body(&fx.plan(json!([{"id": "a", "goal": "Port it", "assignee": worker,
                          "resetContext": true}])));
    assert!(!fx.agents.agent(fx.worker).unwrap().fresh_context_requested);

    body(&fx.dispatch("a"));

    assert!(fx.agents.agent(fx.worker).unwrap().fresh_context_requested);
    assert!(fx.messages.latest_unread_id(fx.worker).is_some(),
            "the task waits in the inbox through the restart");
}

#[test]
fn a_task_without_reset_context_keeps_the_conversation() {
    let mut fx = fixture();
    let worker = fx.worker.to_string();
    body(&fx.plan(json!([{"id": "a", "goal": "Port it", "assignee": worker}])));

    body(&fx.dispatch("a"));

    assert!(!fx.agents.agent(fx.worker).unwrap().fresh_context_requested);
}

/// The caller would restart itself out of the turn that dispatched it.
#[test]
fn a_reset_context_task_for_the_caller_is_refused() {
    let mut fx = fixture();
    let lead = fx.lead.to_string();
    body(&fx.plan(json!([{"id": "a", "goal": "Port it", "assignee": lead,
                          "resetContext": true}])));

    let result = fx.dispatch("a");

    assert_eq!(result.is_error, Some(true));
    assert!(!fx.agents.agent(fx.lead).unwrap().fresh_context_requested);
    assert_eq!(body(&fx.status())["tasks"][0]["state"],
               "ready",
               "a refused dispatch leaves the task to dispatch again");
}

#[test]
fn reset_context_is_reported_on_the_task() {
    let mut fx = fixture();
    let worker = fx.worker.to_string();
    let plan = body(&fx.plan(json!([{"id": "a", "goal": "Port it", "assignee": worker,
                                     "resetContext": true},
                                    {"id": "b", "goal": "Test it", "assignee": worker}])));

    assert_eq!(plan["tasks"][0]["resetContext"], true);
    assert!(plan["tasks"][1].get("resetContext").is_none());
}

/// #384 review: an assignee idle in the middle of another task - waiting
/// on a reply - has read that task already, and a fresh conversation would
/// lose it. The reset is refused until that task is reported.
#[test]
fn a_reset_context_task_waits_for_the_assignees_task_in_flight() {
    let mut fx = fixture();
    let worker = fx.worker.to_string();
    body(&fx.plan(json!([{"id": "a", "goal": "Port it", "assignee": worker},
                         {"id": "b", "goal": "Review it", "assignee": worker,
                          "resetContext": true}])));
    body(&fx.dispatch("a"));

    let refused = fx.dispatch("b");
    assert_eq!(refused.is_error, Some(true));
    assert!(refused.content[0].text.contains("'a'"),
            "the refusal names the task in flight: {}",
            refused.content[0].text);
    assert!(!fx.agents.agent(fx.worker).unwrap().fresh_context_requested);
    assert_eq!(body(&fx.status())["tasks"][1]["state"], "ready");

    body(&fx.complete("a", "done"));
    body(&fx.dispatch("b"));
    assert!(fx.agents.agent(fx.worker).unwrap().fresh_context_requested);
}

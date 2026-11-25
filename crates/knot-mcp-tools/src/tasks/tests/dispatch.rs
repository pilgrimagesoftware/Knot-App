//! `dispatch-task`: delivering a ready task, the refusals (unmet
//! dependency, no plan, no candidate, locked or missing assignee, a second
//! dispatch), and starting a deactivated assignee.

use super::*;

/// "Ready task is delivered".
#[test]
fn a_ready_task_is_delivered_and_marked_dispatched() {
    let mut f = fixture();
    let worker = f.worker;
    f.plan(json!([{"id": "a", "goal": "build the thing", "assignee": "worker"}]));

    let result = f.dispatch("a");

    assert_eq!(body(&result)["recipientId"], worker.to_string());
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "dispatched".to_string())]);
}

/// "Unmet dependency is reported".
#[test]
fn dispatching_behind_an_unmet_dependency_names_it() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"},
                  {"id": "b", "goal": "y", "assignee": "worker", "dependsOn": ["a"]}]));

    let result = f.dispatch("b");

    let text = error_text(&result);
    assert!(text.contains('a'), "{text}");
    assert_eq!(states(&f.status())[1].1, "pending", "b is untouched");
}

/// "A fan-out without a plan is refused".
#[test]
fn dispatching_with_no_committed_plan_says_to_plan_first() {
    let mut f = fixture();

    let result = f.dispatch("a");

    let text = error_text(&result);
    assert!(text.contains("plan-tasks"), "{text}");
}

/// "A tag assignee resolves at dispatch time".
#[test]
fn a_capability_assignee_resolves_against_the_registry_at_dispatch() {
    let mut f = fixture();
    let worker = f.worker;
    f.plan(json!([{"id": "a", "goal": "x", "capabilities": ["rust"]}]));

    let result = f.dispatch("a");

    assert_eq!(body(&result)["recipientId"], worker.to_string());
    assert_eq!(body(&f.status())["tasks"][0]["dispatchedTo"],
               worker.to_string());
}

/// "No candidate for a tag assignee".
#[test]
fn a_capability_no_visible_agent_carries_is_refused_and_leaves_the_task_ready() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "capabilities": ["haskell"]}]));

    let result = f.dispatch("a");

    assert!(error_text(&result).contains("describe-agents"));
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "ready".to_string())]);
}

/// "A rejected delivery does not consume the task". A shell agent cannot
/// receive messages, so the messaging layer refuses and the task must stay
/// dispatchable.
#[test]
fn a_delivery_the_messaging_rules_reject_leaves_the_task_ready() {
    let mut f = fixture();
    let companion = f.agents.create_shell_companion(f.lead).unwrap();
    f.agents.set_registered(companion, true);
    let plan = json!([{"id": "a", "goal": "x", "assignee": companion.to_string()}]);
    f.plan(plan);

    let result = f.dispatch("a");

    assert_eq!(result.is_error, Some(true));
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "ready".to_string())]);
}

/// #563: a dispatch is a direct message to one agent, so it starts a
/// deactivated assignee the way `send-message` does (`task-graph`,
/// "Dispatch to a deactivated assignee starts it").
#[test]
fn a_dispatch_queues_a_deactivated_assignee_to_be_started() {
    let mut f = fixture();
    // `create` leaves an agent deactivated until something starts it.
    assert!(!f.agents.agent(f.worker).unwrap().activated);
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));

    let result = f.dispatch("a");

    assert_eq!(result.is_error, None);
    assert_eq!(&*f.activation.lock(), &[f.worker]);
}

/// A capability assignee resolves to an agent first, and that agent is the
/// one started.
#[test]
fn a_dispatch_by_capability_queues_the_agent_it_resolved_to() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "capabilities": ["rust"]}]));

    let result = f.dispatch("a");

    assert_eq!(result.is_error, None);
    assert_eq!(&*f.activation.lock(), &[f.worker]);
}

#[test]
fn a_dispatch_to_an_activated_assignee_queues_nothing() {
    let mut f = fixture();
    f.agents.set_activated(f.worker, true);
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));

    let result = f.dispatch("a");

    assert_eq!(result.is_error, None);
    assert!(f.activation.lock().is_empty());
}

/// A rejected dispatch delivers nothing, so it starts nobody - the same
/// rule as a rejected direct send.
#[test]
fn a_dispatch_the_messaging_rules_reject_queues_nothing() {
    let mut f = fixture();
    let companion = f.agents.create_shell_companion(f.lead).unwrap();
    f.agents.set_registered(companion, true);
    f.plan(json!([{"id": "a", "goal": "x", "assignee": companion.to_string()}]));

    let result = f.dispatch("a");

    assert_eq!(result.is_error, Some(true));
    assert!(f.activation.lock().is_empty());
}

#[test]
fn a_task_with_no_assignee_cannot_be_dispatched() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x"}]));

    let result = f.dispatch("a");

    assert!(error_text(&result).contains("no assignee"));
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "ready".to_string())]);
}

#[test]
fn a_dispatched_task_cannot_be_dispatched_twice() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));
    f.dispatch("a");

    let result = f.dispatch("a");

    assert!(error_text(&result).contains("dispatched"));
}

/// #310: a locked agent can't be messaged, so it is passed over for a task
/// matched by capability, and the next match takes it.
#[test]
fn a_capability_dispatch_passes_over_a_locked_agent() {
    let mut f = fixture();
    // Named to sort after "worker": candidates are ordered by availability,
    // cost and then name, so the locked agent is the first match.
    let other = agent(&mut f.agents, "zed", &["rust"], Some(f.worker));
    f.agents.set_locked(f.worker, true);
    f.plan(json!([{"id": "a", "goal": "x", "capabilities": ["rust"]}]));

    let result = f.dispatch("a");

    assert_eq!(result.is_error, None, "{}", result.content[0].text);
    assert_eq!(&*f.activation.lock(),
               &[other],
               "the unlocked match is started");
    assert!(f.messages.unread_for(f.worker).is_empty());
}

/// #310: a task assigned to a locked agent by name is refused, and starts
/// nothing.
#[test]
fn a_dispatch_to_a_locked_assignee_is_refused_and_starts_nothing() {
    let mut f = fixture();
    f.agents.set_locked(f.worker, true);
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));

    let result = f.dispatch("a");

    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("locked"),
            "{}",
            result.content[0].text);
    assert!(f.activation.lock().is_empty());
}

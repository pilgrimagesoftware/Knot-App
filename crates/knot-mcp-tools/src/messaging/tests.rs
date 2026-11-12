//! Unit tests for [`super`].

use knot_agents::CreateOptions;
use knot_messaging::NoopNotifier;
use serde_json::json;

use super::*;

fn registered(store: &mut AgentStore, folder: &str) -> uuid::Uuid {
    let id = store.create(folder, CreateOptions::default());
    store.set_registered(id, true);
    id
}

fn activation_queue() -> ActivationQueue {
    std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()))
}

#[test]
fn send_message_succeeds_between_registered_workspace_members() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    let mut messages = MessageStore::new();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              None,
                              &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, None);
    assert!(messages.has_unread(to));
}

#[test]
fn send_message_to_shell_agent_surfaces_rejection_text() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = agents.create("/tmp/shell",
                           CreateOptions { agent_type: Some("shell".to_string()),
                                           insert_after: Some(from),
                                           ..Default::default() });
    agents.set_registered(to, true);
    let mut messages = MessageStore::new();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              None,
                              &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.content[0].text,
               "Cannot send messages to shell agents");
}

#[test]
fn check_messages_marks_read_by_default() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    let mut messages = MessageStore::new();
    send_message(&agents,
                 &mut messages,
                 &NoopNotifier,
                 None,
                 &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    let result = check_messages(&agents, &mut messages, &json!({"agentId": to.to_string()}));

    assert_eq!(result.is_error, None);
    assert!(!messages.has_unread(to));
}

#[test]
fn check_messages_without_mark_as_read_leaves_unread() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    let mut messages = MessageStore::new();
    send_message(&agents,
                 &mut messages,
                 &NoopNotifier,
                 None,
                 &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    check_messages(&agents,
                   &mut messages,
                   &json!({"agentId": to.to_string(), "markAsRead": false}));

    assert!(messages.has_unread(to));
}

#[test]
fn direct_send_queues_a_deactivated_recipient_for_activation() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    // `create` leaves an agent deactivated until something starts it,
    // which is the state this queues on.
    assert!(!agents.agent(to).unwrap().activated);
    let mut messages = MessageStore::new();
    let activation = activation_queue();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              Some(&activation),
                              &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, None);
    assert_eq!(&*activation.lock(), &[to]);
}

#[test]
fn direct_send_to_an_activated_recipient_queues_nothing() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    agents.set_activated(to, true);
    let mut messages = MessageStore::new();
    let activation = activation_queue();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              Some(&activation),
                              &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, None);
    assert!(activation.lock().is_empty());
}

#[test]
fn a_send_rejected_by_a_routing_rule_queues_nothing() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    // Shell recipient: rejected by `send`'s shell-agent rule, after the
    // recipient resolves - the case where a queue push would be easiest
    // to misplace.
    let shell = agents.create("/tmp/shell",
                              CreateOptions { agent_type: Some("shell".to_string()),
                                              insert_after: Some(from),
                                              ..Default::default() });
    agents.set_registered(shell, true);
    let mut messages = MessageStore::new();
    let activation = activation_queue();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              Some(&activation),
                              &json!({"from": from.to_string(), "to": shell.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, Some(true));
    assert!(activation.lock().is_empty());
}

#[test]
fn a_send_to_an_unresolvable_recipient_queues_nothing() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let mut messages = MessageStore::new();
    let activation = activation_queue();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              Some(&activation),
                              &json!({"from": from.to_string(), "to": uuid::Uuid::new_v4().to_string(), "content": "hi"}));

    assert_eq!(result.is_error, Some(true));
    assert!(activation.lock().is_empty());
}

#[test]
fn broadcast_never_queues_a_deactivated_recipient() {
    let mut agents = AgentStore::new();
    let sender = registered(&mut agents, "/tmp/a");
    let deactivated = registered(&mut agents, "/tmp/b");
    assert!(!agents.agent(deactivated).unwrap().activated);
    let mut messages = MessageStore::new();
    let activation = activation_queue();

    let result = broadcast_message(&agents,
                                   &mut messages,
                                   &NoopNotifier,
                                   &json!({"from": sender.to_string(), "content": "hi all"}));

    assert_eq!(result.is_error, None);
    assert!(messages.has_unread(deactivated));
    // `broadcast_message` takes no queue at all - the strongest form of
    // "a broadcast activates nobody" this layer can assert.
    assert!(activation.lock().is_empty());
}

#[test]
fn broadcast_reports_recipient_count() {
    let mut agents = AgentStore::new();
    let sender = registered(&mut agents, "/tmp/a");
    registered(&mut agents, "/tmp/b");
    registered(&mut agents, "/tmp/c");
    let mut messages = MessageStore::new();

    let result = broadcast_message(&agents,
                                   &mut messages,
                                   &NoopNotifier,
                                   &json!({"from": sender.to_string(), "content": "hi all"}));

    assert_eq!(result.is_error, None);
    assert!(result.content[0].text.contains("\"recipientCount\": 2"));
}

/// #310: a locked agent cannot be messaged, and so is not started by a
/// message either.
#[test]
fn a_locked_recipient_is_refused_and_not_started() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    agents.set_locked(to, true);
    let mut messages = MessageStore::new();
    let activation = activation_queue();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              Some(&activation),
                              &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.content[0].text,
               knot_messaging::SendError::RecipientLocked.to_string());
    assert!(activation.lock().is_empty(),
            "a refused message starts nothing");
    assert!(messages.unread_for(to).is_empty());
}

/// #310: a locked agent cannot message others.
#[test]
fn a_locked_sender_is_refused() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    agents.set_locked(from, true);
    let mut messages = MessageStore::new();
    let activation = activation_queue();

    let result = send_message(&agents,
                              &mut messages,
                              &NoopNotifier,
                              Some(&activation),
                              &json!({"from": from.to_string(), "to": to.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.content[0].text,
               knot_messaging::SendError::SenderLocked.to_string());
    assert!(activation.lock().is_empty());
}

/// #349: a locked agent's broadcast is refused with the reason, as its
/// send is - not answered `success: false` with a count of zero, which
/// reads the same as a workspace with nobody else in it.
#[test]
fn a_locked_sender_broadcast_is_refused_with_the_reason() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    agents.set_locked(from, true);
    let mut messages = MessageStore::new();

    let result = broadcast_message(&agents,
                                   &mut messages,
                                   &NoopNotifier,
                                   &json!({"from": from.to_string(), "content": "hi"}));

    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.content[0].text,
               knot_messaging::SendError::SenderLocked.to_string());
    assert!(messages.unread_for(to).is_empty());
}

/// The messages a `check-messages` result lists.
fn listed(result: &ToolCallResult) -> Vec<serde_json::Value> {
    let body: serde_json::Value =
        serde_json::from_str(&result.content[0].text).expect("check-messages answers JSON");
    body["messages"].as_array().cloned().unwrap_or_default()
}

/// #405: a conversation that starts after its brief was read finds it with
/// `includeRead`, marked as already read.
#[test]
fn include_read_returns_a_brief_already_read() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    let mut messages = MessageStore::new();
    send_message(&agents,
                 &mut messages,
                 &NoopNotifier,
                 None,
                 &json!({"from": from.to_string(), "to": to.to_string(), "content": "the brief"}));
    check_messages(&agents, &mut messages, &json!({"agentId": to.to_string()}));

    let plain = check_messages(&agents, &mut messages, &json!({"agentId": to.to_string()}));
    assert!(listed(&plain).is_empty(),
            "read messages stay out by default");

    let all = check_messages(&agents,
                             &mut messages,
                             &json!({"agentId": to.to_string(), "includeRead": true}));
    let found = listed(&all);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["content"], "the brief");
    assert_eq!(found[0]["read"], true);
}

#[test]
fn since_leaves_out_what_was_sent_before_it() {
    let mut agents = AgentStore::new();
    let from = registered(&mut agents, "/tmp/a");
    let to = registered(&mut agents, "/tmp/b");
    let mut messages = MessageStore::new();
    send_message(&agents,
                 &mut messages,
                 &NoopNotifier,
                 None,
                 &json!({"from": from.to_string(), "to": to.to_string(), "content": "old"}));

    let result = check_messages(&agents,
                                &mut messages,
                                &json!({"agentId": to.to_string(),
                                        "includeRead": true,
                                        "since": "2999-01-01T00:00:00Z"}));

    assert_eq!(result.is_error, None);
    assert!(listed(&result).is_empty());
    assert!(messages.has_unread(to),
            "a message filtered out is not marked read");
}

#[test]
fn a_since_that_is_not_a_time_is_an_error() {
    let mut agents = AgentStore::new();
    let to = registered(&mut agents, "/tmp/b");
    let mut messages = MessageStore::new();

    let result = check_messages(&agents,
                                &mut messages,
                                &json!({"agentId": to.to_string(), "since": "yesterday"}));

    assert_eq!(result.is_error, Some(true));
}

use knot_agents::AgentStore;
use knot_mcp::ToolCallResult;
use knot_messaging::{CheckOptions, DeliveryNotifier, MessageStore, broadcast, check, send};

use crate::ActivationQueue;
use crate::args::{optional_bool, optional_str, require_str};
use crate::lookup::{agent_not_found, find_in_workspace, workspace_members};
use crate::responses::{
    BroadcastResponse, CheckMessagesResponse, MessageInfo, SendMessageResponse, success,
};

/// `activation` is where a recipient that was not activated is recorded for
/// the app to start, per `mcp-messaging`'s "Direct send activates a
/// deactivated recipient". `None` outside the app (tests, benches): the
/// message still sends, nothing starts it.
pub fn send_message(agents: &AgentStore, messages: &mut MessageStore,
                    notifier: &dyn DeliveryNotifier, activation: Option<&ActivationQueue>,
                    arguments: &serde_json::Value)
                    -> ToolCallResult {
    let from = match require_str(arguments, "from") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let to = match require_str(arguments, "to") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let content = match require_str(arguments, "content") {
        Ok(v) => v,
        Err(err) => return err,
    };

    let Some(sender) = crate::lookup::find_by_name_or_id(agents, from)
    else {
        return agent_not_found(agents, from);
    };
    let sender = sender.clone();
    let members = workspace_members(agents, sender.id);

    let Some(recipient) = find_in_workspace(agents, sender.id, to)
    else {
        return ToolCallResult::error("Failed to send message: Recipient not found");
    };

    let recipient_id = recipient.id;
    let recipient_activated = recipient.activated;

    match send(messages, notifier, &sender, &members, recipient_id, content) {
        Ok(_) => {
            // Only once the routing rules above have passed - a rejected
            // send activates nobody. The store flag, not session presence,
            // is what `agent-lifecycle` states the rule in terms of.
            if !recipient_activated && let Some(queue) = activation {
                queue.lock().push(recipient_id);
            }
            success(&SendMessageResponse {
            success: true,
            message: "Message sent successfully. Don't check for a response right away - you will be notified when the other agent responds.".to_string(),
        })
        }
        Err(err) => ToolCallResult::error(err.to_string()),
    }
}

pub fn check_messages(agents: &AgentStore, messages: &mut MessageStore,
                      arguments: &serde_json::Value)
                      -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let Some(agent) = crate::lookup::find_by_name_or_id(agents, agent_id_str)
    else {
        return agent_not_found(agents, agent_id_str);
    };
    let since = match optional_str(arguments, "since").map(parse_rfc3339)
                                                      .transpose()
    {
        Ok(since) => since,
        Err(()) => {
            return ToolCallResult::error("`since` must be an RFC 3339 time, as `timestamp` is \
                                          reported (2026-11-12T20:00:00Z)");
        }
    };
    let options =
        CheckOptions { mark_as_read: optional_bool(arguments, "markAsRead").unwrap_or(true),
                       include_read: optional_bool(arguments, "includeRead").unwrap_or(false),
                       since };

    let found = check(messages, agent.id, options);
    let infos = found.into_iter()
                     .map(|m| {
                         let sender_name = agents.agent(m.from)
                                                 .map(|a| a.name.clone())
                                                 .unwrap_or_else(|| m.from.to_string());
                         MessageInfo { id:        m.id.to_string(),
                                       from:      sender_name,
                                       content:   m.content,
                                       timestamp: rfc3339(m.timestamp),
                                       read:      m.is_read, }
                     })
                     .collect();

    success(&CheckMessagesResponse { messages: infos })
}

pub fn broadcast_message(agents: &AgentStore, messages: &mut MessageStore,
                         notifier: &dyn DeliveryNotifier, arguments: &serde_json::Value)
                         -> ToolCallResult {
    let from = match require_str(arguments, "from") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let content = match require_str(arguments, "content") {
        Ok(v) => v,
        Err(err) => return err,
    };

    let Some(sender) = crate::lookup::find_by_name_or_id(agents, from)
    else {
        return agent_not_found(agents, from);
    };
    let sender = sender.clone();
    let members = workspace_members(agents, sender.id);

    // A refused sender is an error, as on `send-message`, not a broadcast
    // that happened to reach nobody (#349).
    match broadcast(messages, notifier, &sender, &members, content) {
        Ok(count) => success(&BroadcastResponse { success:         count > 0,
                                                  recipient_count: count, }),
        Err(err) => ToolCallResult::error(err.to_string()),
    }
}

/// RFC 3339, matching the Swift reference's `ISO8601DateFormatter`.
/// `text` as an instant, in the format [`rfc3339`] writes.
fn parse_rfc3339(text: &str) -> Result<std::time::SystemTime, ()> {
    time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
        .map(std::time::SystemTime::from)
        .map_err(|_| ())
}

fn rfc3339(time: std::time::SystemTime) -> String {
    time::OffsetDateTime::from(time).format(&time::format_description::well_known::Rfc3339)
                                    .unwrap_or_default()
}

#[cfg(test)]
mod tests;

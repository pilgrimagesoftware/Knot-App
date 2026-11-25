//! When an agent's quiet turn earns a nudge or a notice: the idle-time inbox
//! prompt (`mcp-messaging`), the awaiting-input notice, and the desktop
//! notification with its body and the agent its response points back to.

use gpui_kit::SystemNotificationResponse;
use uuid::Uuid;

/// What the idle-time delivery nudge needs to know about one agent, per
/// `mcp-messaging`. Grouped because the decision has several inputs and a
/// five-argument predicate invites callers to transpose two of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NudgeCheck<'a> {
    pub(crate) agent_type:     &'a str,
    pub(crate) mcp_enabled:    bool,
    /// The most recent unread message for this agent, if any.
    pub(crate) latest_message: Option<Uuid>,
    /// The last message this agent was already nudged about.
    pub(crate) last_nudged:    Option<Uuid>,
    /// The agent's state is Idle. A nudge must not land mid-work.
    pub(crate) idle:           bool,
}

// The bool form of `inbox_prompt_message_id`, kept because the spec's
// conditions read as a predicate and the tests assert them that way.
#[allow(dead_code)]
/// Whether to send `agent` the "check your inbox" prompt.
///
/// Every condition here is one the spec names: MCP off means no messaging at
/// all; a shell agent cannot receive messages; an unread message that has
/// already been nudged about must not nudge again on the next poll; and a
/// busy agent is nudged later instead.
///
/// Whether the session can take a prompt *this instant* is deliberately not
/// a condition. That was checked here and the nudge dropped when it failed,
/// which made delivery depend on the poll catching a quiet moment. The
/// delivery path queues instead (`send_inbox_nudge`), so a turn that starts
/// between this check and the send defers the nudge rather than losing it -
/// `mcp-messaging`'s "Inbox nudges preserve interrupted session work".
pub(crate) fn should_inject_inbox_prompt(check: NudgeCheck<'_>) -> bool {
    inbox_prompt_message_id(check).is_some()
}

/// The message id an inbox nudge would be *about*, or `None` when no nudge is
/// due - the same decision [`should_inject_inbox_prompt`] reports as a bool,
/// carrying the value out with it.
///
/// Callers need both the verdict and the id they must record as nudged.
/// Returning it here rather than re-reading `latest_message` after a `true`
/// keeps the two from drifting: the caller used to `expect()` that the field
/// this predicate had checked was still `Some`, which is a panic in the
/// repaint poll the moment a condition above changes without the caller
/// changing with it.
pub(crate) fn inbox_prompt_message_id(check: NudgeCheck<'_>) -> Option<Uuid> {
    if !(check.mcp_enabled && !knot_core::agent_type::is_shell(check.agent_type) && check.idle) {
        return None;
    }
    check.latest_message
         .filter(|message_id| Some(*message_id) != check.last_nudged)
}

/// Whether an agent entering Awaiting input should raise a notice.
///
/// Two suppressions, both named by
/// `openspec/specs/desktop-notifications/spec.md`: the agent the user is
/// already looking at does not need announcing, and a second hook event for
/// a prompt already announced is a repeat, not news.
///
/// An absent message does *not* suppress. The spec's "default body without a
/// message" scenario is explicit that an agent needing attention is worth
/// raising whether or not the hook supplied text - that is what
/// [`AWAITING_INPUT_DEFAULT_BODY`] is for. The ported predicate used to
/// return `false` here, which is why that scenario had never been met.
pub(crate) fn should_show_awaiting_notice(selected_agent: Option<Uuid>, agent_id: Uuid,
                                          message: &str, last_message: Option<&String>)
                                          -> bool {
    selected_agent != Some(agent_id) && last_message.is_none_or(|last| last != message)
}

pub(crate) const AWAITING_INPUT_DEFAULT_BODY: &str = "Needs your attention";

/// Whether a desktop notification should be raised for an agent entering
/// Awaiting input, gating the same "is this a fresh prompt for an agent the
/// user isn't already looking at" signal `should_show_awaiting_notice`
/// computes for the in-window toast behind the
/// `desktop_notifications_enabled` setting.
pub(crate) fn should_notify(desktop_notifications_enabled: bool, show_awaiting_notice: bool)
                            -> bool {
    desktop_notifications_enabled && show_awaiting_notice
}

/// The notification body: the hook-supplied message when non-empty,
/// otherwise a default.
pub(crate) fn notification_body(message: &str) -> &str {
    if message.is_empty() {
        AWAITING_INPUT_DEFAULT_BODY
    }
    else {
        message
    }
}

/// Parses a [`SystemNotificationResponse`]'s tag back into the agent id it
/// was posted for, or `None` if the tag isn't a valid uuid. A click routes
/// to that agent through `notification_click`.
pub(crate) fn notification_response_agent_id(response: &SystemNotificationResponse)
                                             -> Option<Uuid> {
    Uuid::parse_str(&response.tag).ok()
}

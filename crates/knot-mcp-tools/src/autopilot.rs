//! Fires Autopilot classification off a completed turn, and turns the
//! resulting [`knot_autopilot::Effect`] into agent-store and queue writes.
//!
//! The trigger condition (which hook, which agent types, that classification
//! must not block the status response) is
//! `openspec/specs/agent-hooks/spec.md`'s "Autopilot classification trigger"
//! requirement; what a classification *means* is
//! `openspec/specs/autopilot/spec.md`. This module is the seam between them:
//! it reads the last message out of the hook payload, checks whether
//! Autopilot is configured to run at all, and - if so - spawns the
//! (blocking) classification off the current tokio runtime so `status()`
//! returns immediately.

use std::sync::Arc;

use knot_agents::AgentState;
use knot_autopilot::{Classification, Effect};
use parking_lot::Mutex;
use serde_json::Value;
use uuid::Uuid;

use crate::McpToolCatalog;

/// What a landed classification asks a workspace window to do - the
/// agent-id-paired mirror of [`Effect`], minus `None`, which is never
/// queued because there is nothing for a window to act on.
#[derive(Debug, Clone, PartialEq)]
pub enum AutopilotEffect {
    /// Mark the agent as needing input and notify, with no particular
    /// message - `knot-desktop-notifications`'s generic awaiting-input
    /// copy applies, same as an explicit `status: "input"` hook.
    Mark,
    /// Open the decision dialog, carrying the message that prompted it.
    Ask {
        last_message:   String,
        classification: Classification,
    },
    /// Hand this text to the agent as if the user had typed it.
    InjectText(String),
}

impl AutopilotEffect {
    fn from_effect(effect: Effect) -> Option<Self> {
        match effect {
            Effect::None => None,
            Effect::Mark => Some(Self::Mark),
            Effect::Ask { last_message,
                          classification, } => Some(Self::Ask { last_message,
                                                                classification }),
            Effect::InjectText(text) => Some(Self::InjectText(text)),
        }
    }
}

/// One entry per landed classification, drained by the workspace window
/// that owns the agent - the same shape as the awaiting-input and
/// activation queues in `crates/knot/src/app_support.rs`, for the same
/// reason: the tool catalog runs on the MCP server's thread, with no
/// window to call directly.
pub type AutopilotQueue = Arc<Mutex<Vec<(Uuid, AutopilotEffect)>>>;

impl McpToolCatalog {
    #[must_use]
    pub fn with_autopilot_queue(self, queue: AutopilotQueue) -> Self {
        *self.autopilot.lock() = Some(queue);
        self
    }

    /// Reads the last assistant message out of a completed turn and, if
    /// Autopilot is enabled and configured, spawns classification for it.
    ///
    /// `hook` is the Claude hook name (`"Stop"` is the only one that
    /// triggers); for Codex every call here is already a turn-complete
    /// event, so `hook` is ignored for that agent type.
    pub(crate) fn maybe_trigger_autopilot(&self, id: Uuid, agent_type: &str, hook: Option<&str>,
                                          payload: &Value) {
        let Some(last_message) = last_message_for_trigger(self, id, agent_type, hook, payload)
        else {
            return;
        };
        if last_message.is_empty() {
            return;
        }
        // Classification is a blocking HTTP call; with no tokio runtime to
        // spawn it on (the tool catalog is built and exercised without one
        // in tests and benches), there is nowhere to run it.
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let Some(queue) = self.autopilot.lock().clone()
        else {
            return;
        };
        let Some(settings) = self.settings.lock().clone()
        else {
            return;
        };
        let snapshot = settings.read();
        if !snapshot.autopilot_enabled || snapshot.ai_api_key.trim().is_empty() {
            return;
        }
        let provider = snapshot.ai_provider;
        let api_key = snapshot.ai_api_key.clone();
        let action = snapshot.autopilot_action;
        let custom_prompt = snapshot.autopilot_custom_prompt.clone();
        let agents = Arc::clone(&self.agents);
        // Mark and Ask both notify through the same path an explicit
        // `status: "input"` hook uses - see `AgentHookHandler::status`'s
        // `hook_input_message` branch - so a landed classification needs no
        // notification text of its own. Captured now, not read again inside
        // the task: a notifier set after this hook fired must not apply to it.
        let awaiting_input = self.awaiting_input.lock().clone();
        tokio::spawn(async move {
            let effect = tokio::task::spawn_blocking(move || {
                             knot_autopilot::analyze(&last_message,
                                                     provider,
                                                     &api_key,
                                                     action,
                                                     &custom_prompt)
                         }).await
                           .unwrap_or(Effect::None);
            let Some(effect) = AutopilotEffect::from_effect(effect)
            else {
                return;
            };
            agents.lock().set_state(id, AgentState::Input);
            if matches!(effect, AutopilotEffect::Mark | AutopilotEffect::Ask { .. })
               && let Some(awaiting_input) = awaiting_input
            {
                awaiting_input.lock().push((id, None));
            }
            queue.lock().push((id, effect));
        });
    }
}

/// The last assistant message to classify, per
/// `openspec/specs/agent-hooks/spec.md`: for Claude, read from the
/// transcript named in the agent's `transcript_path` metadata on a `Stop`
/// hook; for Codex, read directly from `payload["last-assistant-message"]`.
/// `None` when this hook/agent combination does not trigger Autopilot at
/// all (as opposed to triggering it with nothing to classify).
fn last_message_for_trigger(catalog: &McpToolCatalog, id: Uuid, agent_type: &str,
                            hook: Option<&str>, payload: &Value)
                            -> Option<String> {
    match agent_type {
        "claude" => {
            if hook != Some("Stop") {
                return None;
            }
            let transcript_path = catalog.agents
                                         .lock()
                                         .agent(id)?
                                         .metadata
                                         .get("transcript_path")?
                                         .clone();
            knot_mcp::last_assistant_message_from_transcript(transcript_path)
        }
        "codex" => payload.get("last-assistant-message")
                          .and_then(Value::as_str)
                          .map(str::to_string),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use knot_agents::AgentStore;
    use knot_messaging::NoopNotifier;
    use serde_json::json;
    use tokio::sync::watch;

    use super::*;

    fn catalog() -> McpToolCatalog {
        let (_tx, rx) = watch::channel(Vec::new());
        McpToolCatalog::new(Arc::new(Mutex::new(AgentStore::new())),
                            rx,
                            Arc::new(NoopNotifier))
    }

    #[test]
    fn unknown_agent_type_never_triggers() {
        let catalog = catalog();
        assert_eq!(last_message_for_trigger(&catalog, Uuid::new_v4(), "gemini", None, &json!({})),
                   None);
    }

    #[test]
    fn claude_requires_stop_hook() {
        let catalog = catalog();
        assert_eq!(last_message_for_trigger(&catalog,
                                            Uuid::new_v4(),
                                            "claude",
                                            Some("PreToolUse"),
                                            &json!({})),
                   None);
    }

    #[test]
    fn codex_reads_payload_directly_no_hook_needed() {
        let catalog = catalog();
        let payload = json!({"last-assistant-message": "should I proceed?"});
        assert_eq!(last_message_for_trigger(&catalog, Uuid::new_v4(), "codex", None, &payload),
                   Some("should I proceed?".to_string()));
    }

    #[test]
    fn codex_missing_field_yields_none() {
        let catalog = catalog();
        assert_eq!(last_message_for_trigger(&catalog, Uuid::new_v4(), "codex", None, &json!({})),
                   None);
    }
}

//! A click on a desktop notification takes the user to what it was about
//! (#1): Knot comes to the front, and an agent's notification opens - or
//! raises - the window of the workspace holding that agent, with the agent
//! selected and shown, as the Swift reference's `switchToAgent` did.
//!
//! Contract: `openspec/specs/desktop-notifications/spec.md`, "Clicking a
//! notification navigates to its agent".
//!
//! An agent's notification carries the agent's id as its tag
//! (`app_state::notification_response_agent_id`). The MCP server's failure
//! notice is not about an agent and carries a tag that deliberately parses
//! as none; it, and a notification for an agent removed since, only bring
//! Knot forward.

use std::sync::Arc;

use gpui_kit::App;
use parking_lot::Mutex;

use crate::app_state::notification_response_agent_id;
use crate::workspace_window::WorkspaceWindow;

/// Routes every notification click from here on.
pub(crate) fn install(store: Arc<Mutex<knot_agents::AgentStore>>,
                      messages: Arc<Mutex<knot_messaging::MessageStore>>, cx: &mut App) {
    cx.on_system_notification_response(move |response, cx| {
          cx.activate(true);
          if let Some(id) = notification_response_agent_id(&response) {
              WorkspaceWindow::open_agent(Arc::clone(&store), Arc::clone(&messages), id, cx);
          }
      });
}

#[cfg(test)]
mod tests;

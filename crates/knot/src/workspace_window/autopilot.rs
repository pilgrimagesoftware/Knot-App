//! Drains the Autopilot effect queue and carries out whatever landed.
//!
//! Contract: `openspec/specs/autopilot/spec.md`. The MCP tool catalog
//! pushes `(agent id, effect)` onto `app_support::Autopilot` from
//! `knot-mcp-tools::autopilot::maybe_trigger_autopilot`, already having set
//! the agent's state to `Input` (see that function) - this is only the
//! part that needs a window: marking raises the desktop notification the
//! `AwaitingInput` path already does, asking opens the decision dialog, and
//! injecting hands text to the agent as if the user had typed it.

use gpui_kit::Context;
use knot_agents::AgentState;
use knot_mcp_tools::AutopilotEffect;
use uuid::Uuid;

use crate::app_support::Autopilot;
use crate::autopilot_decision_sheet::open_autopilot_decision_sheet;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Drains the entries this window owns, applying each. Entries for
    /// other workspaces are put back, same as
    /// [`Self::raise_awaiting_notifications`].
    pub(in crate::workspace_window) fn drain_autopilot_effects(&mut self, cx: &mut Context<Self>)
                                                               -> bool {
        if !cx.has_global::<Autopilot>() {
            return false;
        }
        let mine = self.workspace_agent_ids();
        let claimed = {
            let queue = cx.global::<Autopilot>().0.clone();
            let mut queue = queue.lock();
            let (claimed, rest): (Vec<_>, Vec<_>) =
                std::mem::take(&mut *queue).into_iter()
                                           .partition(|(id, _)| mine.contains(id));
            *queue = rest;
            claimed
        };
        if claimed.is_empty() {
            return false;
        }
        for (id, effect) in claimed {
            self.apply_autopilot_effect(id, effect, cx);
        }
        true
    }

    fn apply_autopilot_effect(&mut self, id: Uuid, effect: AutopilotEffect,
                              cx: &mut Context<Self>) {
        match effect {
            // The agent-store write and the awaiting-input queue push both
            // already happened before this effect was queued (see
            // `knot-mcp-tools::autopilot::maybe_trigger_autopilot`); marking
            // has no further UI effect of its own -
            // `raise_awaiting_notifications` raises the notification from
            // that queue push, same as an explicit `status: "input"` hook.
            AutopilotEffect::Mark => {}
            AutopilotEffect::Ask { last_message,
                                   classification, } => {
                let Some(name) = self.store.lock().agent(id).map(|agent| agent.name.clone())
                else {
                    return;
                };
                let store = self.store.clone();
                let view = cx.entity();
                open_autopilot_decision_sheet(
                    name,
                    last_message,
                    classification,
                    {
                        let store = store.clone();
                        move |_window, _app| store.lock().set_state(id, AgentState::Idle)
                    },
                    {
                        let view = view.clone();
                        move |window, app| {
                            view.update(app, |view, cx| view.reveal_agent(id, cx));
                            window.activate_window();
                        }
                    },
                    (classification == knot_autopilot::Classification::Binary).then(|| {
                        let view = view.clone();
                        move |_window: &mut gpui_kit::Window, app: &mut gpui_kit::App| {
                            view.update(app, |view, cx| {
                                view.deliver_prompt(id, "yes, continue");
                                cx.notify();
                            });
                        }
                    }),
                    cx,
                );
            }
            AutopilotEffect::InjectText(text) => {
                self.deliver_prompt(id, &text);
            }
        }
    }
}

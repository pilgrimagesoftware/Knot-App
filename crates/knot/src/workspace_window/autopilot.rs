//! Autopilot in a workspace window: triggering it when a Panel-mode agent's
//! turn ends, and carrying out whatever a classification lands.
//!
//! Contract: `openspec/specs/autopilot/spec.md`. Two triggers feed one queue.
//! A Terminal-mode agent's turn ends on the agent-hook route, in
//! `knot-mcp-tools`; a Panel-mode agent's ends here, read off its ACP
//! session, because no hook reports it. Both go through
//! `knot_mcp_tools::spawn_autopilot`, which classifies off the main thread
//! and pushes `(agent id, effect)` onto `app_support::Autopilot` - already
//! having marked the agent awaiting input when the effect asks for the user.
//! What is left for a window is the part that needs one: the decision dialog,
//! delivering a reply, and saying in the conversation what Autopilot did.

use std::sync::Arc;

use gpui_kit::Context;
use knot_agents::AgentState;
use knot_mcp_tools::{AutopilotEffect, AutopilotSinks};
use uuid::Uuid;

use crate::app_support::{Autopilot, AwaitingInput};
use crate::autopilot_decision_sheet::open_autopilot_decision_sheet;
use crate::panel_session;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

impl WorkspaceWindow {
    /// Starts Autopilot on `id`'s reply now that its panel turn has ended,
    /// when Autopilot is on and configured - `spawn_autopilot` checks both.
    ///
    /// The reply is read here, on the main thread, because the conversation
    /// it lives in is this window's; only the classification leaves.
    pub(in crate::workspace_window) fn trigger_panel_autopilot(&self, id: Uuid,
                                                               cx: &Context<Self>) {
        let (Some(queue), Some(reply)) = (cx.try_global::<Autopilot>(), self.panel_reply(id))
        else {
            return;
        };
        let sinks = AutopilotSinks { agents:         Arc::clone(&self.store),
                                     queue:          queue.0.clone(),
                                     awaiting_input: cx.try_global::<AwaitingInput>()
                                                       .map(|awaiting| awaiting.0.clone()), };
        // `spawn_autopilot` spawns onto the current runtime; this window's
        // is the one its sessions already run on.
        let _runtime = self.runtime.enter();
        knot_mcp_tools::spawn_autopilot(id, reply, &crate::settings_global::read(cx), sinks);
    }

    /// The last reply in `id`'s conversation, if it has a ready session and
    /// the turn left one.
    fn panel_reply(&self, id: Uuid) -> Option<String> {
        let handle = self.ready_panel_state(id)?;
        let state = handle.lock();
        state.last_reply().map(str::to_owned)
    }

    /// `id`'s conversation, if its panel session is ready.
    fn ready_panel_state(&self, id: Uuid)
                         -> Option<Arc<parking_lot::Mutex<crate::panel_state::PanelState>>> {
        let slot = self.panel_sessions.get(&id)?.lock();
        match &*slot {
            panel_session::PanelSessionSlot::Ready(handle) => Some(handle.state()),
            _ => None,
        }
    }

    /// Adds `key`'s text to `id`'s conversation as a notice, when it has
    /// one. A Terminal-mode agent has no conversation to add it to.
    fn autopilot_notice(&self, id: Uuid, key: &str) {
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return;
        };
        if let panel_session::PanelSessionSlot::Ready(handle) = &*slot.lock() {
            handle.record_notice(knot_core::l10n::t(key));
        }
    }

    /// Sends `text` to `id` as Autopilot's reply, with a notice above it
    /// saying who wrote it.
    ///
    /// A panel session takes it as a prompt like any other, queued behind a
    /// running turn. A terminal gets it typed *and submitted*: the terminal
    /// fallback in [`Self::deliver_prompt`] types without pressing Return,
    /// which suits a broadcast the user may want to edit but left an
    /// Autopilot reply sitting unsent at the prompt.
    pub(in crate::workspace_window) fn deliver_autopilot_reply(&mut self, id: Uuid, text: &str)
                                                               -> bool {
        self.autopilot_notice(id, "autopilot.notice.replied");
        if self.deliver_panel_prompt(id, text.to_string(), PromptOrigin::Autopilot) {
            return true;
        }
        let Some(session) = self.sessions.get(&id)
        else {
            return false;
        };
        match session.lock().send_command(text) {
            Ok(()) => true,
            Err(error) => {
                eprintln!("failed to deliver Autopilot's reply to agent {id}: {error}");
                false
            }
        }
    }

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

    pub(in crate::workspace_window) fn apply_autopilot_effect(&mut self, id: Uuid,
                                                              effect: AutopilotEffect,
                                                              cx: &mut Context<Self>) {
        match effect {
            // The agent-store write and the awaiting-input queue push both
            // already happened before this effect was queued (see
            // `knot_mcp_tools::land`), and `raise_awaiting_notifications`
            // raises the notification from that push. What only a window can
            // add is the line in the conversation saying it was Autopilot,
            // not the agent, that stopped here.
            AutopilotEffect::Mark(_) => self.autopilot_notice(id, "autopilot.notice.mark"),
            AutopilotEffect::Ask { last_message,
                                   classification, } => {
                let Some(name) = self.store.lock().agent(id).map(|agent| agent.name.clone())
                else {
                    return;
                };
                self.autopilot_notice(id, "autopilot.notice.ask");
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
                                view.deliver_autopilot_reply(id, "yes, continue");
                                cx.notify();
                            });
                        }
                    }),
                    cx,
                );
            }
            AutopilotEffect::InjectText(text) => {
                self.deliver_autopilot_reply(id, &text);
            }
        }
    }
}

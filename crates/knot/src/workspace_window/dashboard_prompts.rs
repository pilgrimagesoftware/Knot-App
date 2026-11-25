//! The workspace dashboard's quick prompts (#10): the cards' fields, and
//! sending from one. This window owns the agents' sessions, so it sends to
//! itself; the Command Center comes through `quick_prompt` from outside.

use gpui_kit::Context;
use gpui_kit::Window;
use uuid::Uuid;

use crate::dashboard::DashboardAgent;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Gives this dashboard's cards their fields; see
    /// `dashboard_quick_prompt::attach`.
    pub(super) fn attach_quick_prompts(&mut self, agents: &mut [DashboardAgent],
                                       window: &mut Window, cx: &mut Context<Self>) {
        let mut fields = std::mem::take(&mut self.dashboard_quick_prompts);
        crate::dashboard_quick_prompt::attach(&mut fields,
                                              agents.iter_mut(),
                                              window,
                                              cx,
                                              Self::send_dashboard_prompt);
        self.dashboard_quick_prompts = fields;
    }

    /// Enter in `id`'s card field: sends the text as the user's prompt -
    /// now, or queued - and clears the field once it is taken. With
    /// `open` (Cmd+Enter) it also opens the agent, as a tap on its card
    /// does. Text nothing could take stays, so a send that raced the session
    /// ending loses nothing.
    fn send_dashboard_prompt(&mut self, id: Uuid, open: bool, window: &mut Window,
                             cx: &mut Context<Self>) {
        let Some(input) = self.dashboard_quick_prompts.get(id).cloned()
        else {
            return;
        };
        let text = input.read(cx).value().to_string();
        if text.trim().is_empty() || !self.quick_prompt(id, &text) {
            return;
        }
        input.update(cx, |field, cx| field.set_value("", window, cx));
        if open {
            self.select_agent(id, false, cx);
            self.view_mode = WorkspaceViewMode::Terminal;
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests;

//! The notice over an agent's pane when Knot refused its MCP requests
//! (`openspec/changes/mcp-auth/design.md` D10).
//!
//! An adapter that stops sending the credential Knot gave it loses Knot's
//! tools to 401s rather than failing loudly. The server counts refused
//! requests against the agent its URL names until the agent next
//! authenticates; `poll_mcp_unavailable` copies which agents that applies to
//! into the window on its repaint poll, and this only draws what is there.

use gpui_kit::AnyElement;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::div;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// `pane`, with the notice above it when `agent`'s MCP requests are being
    /// refused.
    pub(in crate::workspace_window) fn with_mcp_banner(&self, agent: Uuid, pane: AnyElement,
                                                       cx: &mut Context<Self>)
                                                       -> AnyElement {
        if !self.mcp_unavailable.contains(&agent) {
            return pane;
        }
        let banner =
            h_flex().id(("mcp-refused-banner", agent.as_u128() as u64))
                    .debug_selector(|| "mcp-refused-banner".into())
                    .flex_shrink_0()
                    .w_full()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().muted)
                    .text_sm()
                    .child(Icon::new(IconName::TriangleAlert).small()
                                                             .text_color(cx.theme().danger))
                    .child(div().flex_1()
                                .min_w_0()
                                .child(knot_core::l10n::t("panel.mcp_refused")));
        v_flex().size_full()
                .child(banner)
                .child(div().flex_1().min_h_0().child(pane))
                .into_any_element()
    }

    /// Copies which of this workspace's agents have had MCP requests refused
    /// since they last authenticated, answering whether that changed - the
    /// server marks it on its own thread, with no context to notify from.
    /// Recomputed from the workspace's agents each time, so an agent that
    /// has gone leaves no entry behind.
    pub(in crate::workspace_window) fn poll_mcp_unavailable(&mut self) -> bool {
        let refusals = &self.mcp_credentials.refusals;
        let now: std::collections::BTreeSet<Uuid> =
            self.workspace_agent_ids()
                .into_iter()
                .filter(|agent| refusals.tools_unavailable(*agent))
                .collect();
        if now == self.mcp_unavailable {
            return false;
        }
        self.mcp_unavailable = now;
        true
    }
}

#[cfg(test)]
mod tests;

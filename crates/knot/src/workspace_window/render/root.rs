//! `Render for WorkspaceWindow`: the dispatcher that snapshots the frame,
//! asks each region's module for its element, and lays the two resizable
//! columns out under the root's actions and key handlers.

use std::sync::Arc;

use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::TitleBar;
use gpui_kit::component::resizable::ResizableState;
use gpui_kit::component::resizable::h_resizable;
use gpui_kit::component::resizable::resizable_panel;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;

use super::sidebar_column::SidebarRows;
use crate::app_bootstrap::PanelOpenPermissionSelector;
use crate::app_bootstrap::PanelPermissionAllow;
use crate::app_bootstrap::PanelPermissionAllowAlways;
use crate::app_bootstrap::PanelPermissionDeny;
use crate::workspace_window::SidebarMenuTargets;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::input::PERMISSION_SELECTOR_ID;
use crate::workspace_window::sidebar_is_compact;
use crate::workspace_window::with_agents_menu_actions;

impl Render for WorkspaceWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // One read for the frame, not one per value: a write landing between
        // two reads would draw a font name from either side of it. Cheap
        // enough to take here rather than thread down - it is a refcount
        // bump, which is the whole reason the surface is copy-on-write and
        // not a mutex.
        let settings = crate::settings_global::read(cx);
        // The title font (Manrope) applies explicitly to header and cell text
        // that isn't the agent's name - the name keeps the app-wide UI font
        // (Adamina), so it needs no override here.
        let title_font_name = settings.title_font_name.clone();
        let title_font_size = px(settings.title_font_size as f32);
        let Some((window_title, agents)) = self.frame_snapshot(cx)
        else {
            return v_flex().size_full()
                           .child(TitleBar::new().border_color(gpui_kit::transparent_black()))
                           .child(knot_core::l10n::t("workspace.missing"));
        };
        // The OS title (Window menu, Cmd+`, Mission Control) has to follow a
        // rename too, and `open.rs` sets it once, from the name the workspace
        // had at open. Guarded on the last value written rather than set
        // every frame: this crosses into AppKit, and a sidebar drag renders
        // continuously.
        if self.titled_as != window_title {
            window.set_window_title(&window_title);
            self.titled_as = window_title.clone();
        }
        let is_dashboard = self.view_mode == WorkspaceViewMode::Dashboard;
        let is_changes = self.view_mode == WorkspaceViewMode::Changes;
        // Every takeover hides the selected agent's header and pane, so the
        // question the rest of this render asks is "is anything taking the
        // content over", not "is it the dashboard".
        let is_takeover = self.view_mode.is_takeover();

        self.prepare_frame(is_takeover, window, cx);
        // Gated on the view inside: nothing is fetched, and nothing expires,
        // while it is closed.
        self.refresh_pull_request_states(cx);
        // The same gate for the Issues and OpenSpec tabs' data.
        self.refresh_work_items(cx);
        // Only while a Codex agent is in the window (#291).
        self.refresh_codex_usage();
        // The one place the compact breakpoint is read. Every surface that
        // changes below it takes this `bool`, so none of them can disagree
        // about where compact begins.
        let sidebar_width = self.sidebar_width(cx);
        let compact = sidebar_is_compact(sidebar_width);

        let agent_rows = self.agent_rows(agents,
                                         title_font_name.clone(),
                                         title_font_size,
                                         compact,
                                         cx);
        let dashboard_row = self.dashboard_row(is_dashboard, compact, cx);
        let pull_requests_row = Some(self.pull_requests_row(compact, cx));

        let selected_header = self.selected_agent_header();

        // One content slot: at most one takeover shows at a time, so the
        // first that claims it wins and `content_column` needs no third arm.
        let takeover_content = self.dashboard_content(is_dashboard, window, cx)
                                   .or_else(|| self.changes_content(is_changes, window, cx));

        let title_bar_left = self.title_bar_left(is_takeover,
                                                 &selected_header,
                                                 &title_font_name,
                                                 title_font_size,
                                                 cx);
        let title_bar_right = self.title_bar_right(is_takeover,
                                                   &selected_header,
                                                   &title_font_name,
                                                   title_font_size,
                                                   cx);
        let selected_menu = self.selected_agent_menu(cx);
        let shortcuts = self.shortcut_availability();
        let file_finder = self.render_file_finder(cx);
        let background_targets = SidebarMenuTargets { store:         Arc::clone(&self.store),
                                                      window_entity: cx.entity(),
                                                      workspace_id:  self.workspace_id, };
        h_flex()
            .size_full()
            .map(|el| with_agents_menu_actions(el, selected_menu.as_ref()))
            .map(|el| Self::with_shortcut_actions(el, shortcuts, cx))
            // On the root element, so a hold registers wherever focus is in
            // the window - the composer and the terminal both let modifier
            // changes and key-downs through. Capture phase for the key-down:
            // a focused input handles most keys and stops them bubbling.
            .on_modifiers_changed(cx.listener(|view, event, _, cx| {
                view.key_hints_modifiers_changed(event, cx)
            }))
            .capture_key_down(cx.listener(|view, event, _, cx| view.key_hints_key_down(event, cx)))
            .track_focus(&self.root_focus)
            .on_action(cx.listener(|view, _: &PanelPermissionAllow, _, cx| {
                view.answer_selected_permission(knot_acp::PermissionDecision::Allow);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &PanelPermissionAllowAlways, _, cx| {
                view.answer_selected_permission(knot_acp::PermissionDecision::AllowAlways);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &PanelPermissionDeny, _, cx| {
                view.answer_selected_permission(knot_acp::PermissionDecision::Deny);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &PanelOpenPermissionSelector, _, cx| {
                if view.selected_agent.is_some_and(|id| {
                    view.store.lock().agent(id).map(|agent| agent.view_mode) == Some(knot_core::ViewMode::Panel)
                }) {
                    view.open_config_selector = Some(PERMISSION_SELECTOR_ID);
                    cx.notify();
                }
            }))
            .child(
                // The two columns are the two panels of a resizable group,
                // so the boundary between them is a divider the user drags.
                // The group's handle is absolutely positioned and takes no
                // layout width, which is what keeps the alignment the
                // sidebar column's comment below depends on.
                h_resizable("workspace-columns")
                    .with_state(&self.sidebar_resize)
                    .on_resize(cx.listener(|view, state: &Entity<ResizableState>, _window, cx| {
                        let Some(width) = state.read(cx)
                                               .sizes()
                                               .first()
                                               .map(|width| f64::from(f32::from(*width)))
                        else {
                            return;
                        };
                        view.persist_sidebar_width(width, cx);
                    }))
                    .child(resizable_panel()
                        .size(px(sidebar_width as f32))
                        .size_range(px(knot_core::consts::SIDEBAR_WIDTH_MIN as f32)
                                    ..px(knot_core::consts::SIDEBAR_WIDTH_MAX as f32))
                        // The panel grows by default; a sized panel beside a
                        // flexible one has to opt out or it takes the slack
                        // back on the frame after a drag.
                        .flex_none()
                        .child(self.sidebar_column(&window_title,
                                                   compact,
                                                   SidebarRows { dashboard_row,
                                                                 pull_requests_row,
                                                                 agent_rows },
                                                   background_targets,
                                                   cx)))
                    .child(resizable_panel().child(self.content_column(is_takeover,
                                                                       takeover_content,
                                                                       title_bar_left,
                                                                       title_bar_right,
                                                                       window,
                                                                       cx))),
            )
            // Last, so it paints over everything else (#5).
            .relative()
            .children(file_finder)
    }
}

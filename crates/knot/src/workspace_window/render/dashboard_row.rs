//! The workspace overview row pinned above the agent list.

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;

use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::key_hints::with_key_hint;

impl WorkspaceWindow {
    /// The workspace overview row, pinned above the agent list.
    ///
    /// `compact` drops its label, leaving the icon - the same trade the agent
    /// rows above it make at the same width.
    pub(super) fn dashboard_row(&self, is_dashboard: bool, compact: bool, cx: &mut Context<Self>)
                                -> gpui_kit::AnyElement {
        // The dashboard sits at the top of the agent list, the way the
        // Swift reference's `overviewRow` does (`SidebarView.swift`), and
        // not as an icon in the bottom bar: it is the workspace's overview
        // of every agent, so it belongs above them, shaped like the rows it
        // summarises. As a bare icon beside "New agent" it read as a minor
        // control and went unnoticed.
        let hint = self.key_hints.shown().map(|hints| hints.dashboard.clone());
        let row = div().id("workspace-dashboard-row");
        let row = if compact {
            row
        }
        else {
            with_key_hint(row, hint.as_deref(), false, cx)
        };
        row.debug_selector(|| "workspace-dashboard-row".into())
           .cursor_pointer()
           .rounded(cx.theme().radius)
           .p_2()
           .bg(if is_dashboard {
               cx.theme().muted
           }
           else {
               cx.theme().transparent
           })
           .child(h_flex().w_full()
                          .gap_3()
                          .items_center()
                          .when(compact, |row| row.justify_center())
                          .child({
                              let icon =
                                  div().w(px(40.))
                                       .h(px(40.))
                                       .flex_shrink_0()
                                       .flex()
                                       .items_center()
                                       .justify_center()
                                       .child(Icon::default().path("icons/layout-dashboard.svg"));
                              if compact {
                                  with_key_hint(icon, hint.as_deref(), true, cx)
                              }
                              else {
                                  icon
                              }
                          })
                          .when(!compact, |row| {
                              row.child(div().flex_1()
                                             .min_w_0()
                                             .font_semibold()
                                             .child(knot_core::l10n::t("dashboard.title")))
                          }))
           .on_click(cx.listener(|view, _: &ClickEvent, _window, cx| {
                           view.view_mode = match view.view_mode {
                               WorkspaceViewMode::Dashboard => WorkspaceViewMode::Terminal,
                               _ => WorkspaceViewMode::Dashboard,
                           };
                           cx.notify();
                       }))
           .into_any_element()
    }
}

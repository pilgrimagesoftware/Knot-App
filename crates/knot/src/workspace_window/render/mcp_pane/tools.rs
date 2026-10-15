//! A server row's tools (#40): the control that opens them, and the list -
//! or what the list says when it is being fetched or cannot be.
//!
//! One line per tool, name then description, each cut to the row's width
//! with the whole description behind a tooltip: a tool's description is
//! written for a model and often runs to paragraphs.

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div,
};
use knot_mcp_client::Tool;

use crate::app_support::single_line;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::mcp_panel::rows::SectionRow;
use crate::workspace_window::mcp_panel::tools::{ToolsKey, ToolsState, Unlisted};

impl WorkspaceWindow {
    /// Whether `row`'s tools are open in the shown agent's section.
    fn mcp_tools_open(&self, row: &SectionRow) -> bool {
        self.selected_agent
            .and_then(|agent_id| self.mcp_section(agent_id))
            .is_some_and(|section| section.tools_open(&ToolsKey::of(row)))
    }

    /// The chevron at the start of a row that opens and shuts its tools.
    pub(super) fn mcp_tools_toggle(&self, index: usize, row: &SectionRow, cx: &mut Context<Self>)
                                   -> gpui_kit::AnyElement {
        let open = self.mcp_tools_open(row);
        let tooltip = knot_core::l10n::t(if open {
                                             "mcp.tools_hide"
                                         }
                                         else {
                                             "mcp.tools_show"
                                         });
        let row = row.clone();
        div().id(("mcp-tools-toggle", index as u64))
             .p_1()
             .rounded_sm()
             .cursor_pointer()
             .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
             .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                             if let Some(agent_id) = view.selected_agent {
                                 view.toggle_mcp_tools(agent_id, &row);
                                 cx.notify();
                             }
                         }))
             .child(Icon::new(if open {
                                  IconName::ChevronDown
                              }
                              else {
                                  IconName::ChevronRight
                              }).size_3()
                                .text_color(cx.theme().muted_foreground))
             .into_any_element()
    }

    /// `row`'s tools, or `None` while they are shut.
    pub(super) fn mcp_tools_block(&self, index: usize, row: &SectionRow, cx: &mut Context<Self>)
                                  -> Option<gpui_kit::AnyElement> {
        if !self.mcp_tools_open(row) {
            return None;
        }
        let state = self.selected_agent
                        .and_then(|agent_id| self.mcp_section(agent_id))
                        .and_then(|section| section.tools(&ToolsKey::of(row)).cloned());
        let muted = cx.theme().muted_foreground;
        let note = |text: String, color| {
            div().text_xs()
                 .text_color(color)
                 .child(text)
                 .into_any_element()
        };
        let body = match state {
            None | Some(ToolsState::Listing) => {
                note(knot_core::l10n::t("mcp.tools_listing"), muted)
            }
            Some(ToolsState::Listed(tools)) if tools.is_empty() => {
                note(knot_core::l10n::t("mcp.tools_none"), muted)
            }
            Some(ToolsState::Listed(tools)) => {
                v_flex().w_full()
                        .children(tools.iter()
                                       .enumerate()
                                       .map(|(at, tool)| tool_line(index, at, tool, cx)))
                        .into_any_element()
            }
            Some(ToolsState::Failed(error)) => note(knot_core::l10n::t_with("mcp.tools_failed",
                                                                            &[("error", &error)]),
                                                    cx.theme().danger),
            Some(ToolsState::Unlisted(Unlisted::NotConnected)) => {
                note(knot_core::l10n::t("mcp.tools_not_connected"), muted)
            }
            Some(ToolsState::Unlisted(Unlisted::NoLaunch)) => {
                note(knot_core::l10n::t("mcp.tools_no_launch"), muted)
            }
        };
        // Indented under the name, past the chevron, so the list reads as
        // the row's own.
        Some(div().pl_6().py_1().w_full().child(body).into_any_element())
    }
}

/// One tool: its name, and its description cut to the line.
fn tool_line(row: usize, at: usize, tool: &Tool, cx: &mut Context<WorkspaceWindow>)
             -> gpui_kit::AnyElement {
    let description = tool.description.clone().unwrap_or_default();
    let full = SharedString::from(description.clone());
    h_flex().id(SharedString::from(format!("mcp-tool-{row}-{at}")))
            .w_full()
            .min_w_0()
            .gap_2()
            .text_xs()
            .when(!full.is_empty(), move |line| {
                line.tooltip(move |window, cx| Tooltip::new(full.clone()).build(window, cx))
            })
            .child(div().flex_shrink_0()
                        .font_family(cx.theme().mono_font_family.clone())
                        .child(single_line(&tool.name)))
            .child(div().min_w_0()
                        .flex_1()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_color(cx.theme().muted_foreground)
                        .child(single_line(&description)))
            .into_any_element()
}

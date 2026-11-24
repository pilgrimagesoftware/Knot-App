//! The sidebar column the frame assembles: its title bar, the scrolling
//! agent list with its drag-to-end filler, and the New Agent row. The rows
//! themselves come from `sidebar` and `sidebar_compact`.

use std::rc::Rc;

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::TitleBar;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;

use crate::app_support::app_titlebar_icon;
use crate::window_options;
use crate::workspace_window::SidebarMenuTargets;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::key_hints::with_key_hint;
use crate::workspace_window::sidebar_background_context_menu;

/// The rows `render` has already built by the time it hands the sidebar
/// over, in the order they are stacked.
///
/// One struct rather than three parameters because `sidebar_column` also
/// takes the window's title, the compact flag and the background menu's
/// targets, and the three prebuilt rows are the group that belongs together.
pub(super) struct SidebarRows {
    pub(super) dashboard_row:     gpui_kit::AnyElement,
    pub(super) pull_requests_row: Option<gpui_kit::AnyElement>,
    pub(super) agent_rows:        Vec<gpui_kit::AnyElement>,
}

impl WorkspaceWindow {
    /// The sidebar's own title bar, which owns the traffic lights.
    ///
    /// It names the workspace, not the application: several of these windows
    /// can be open at once and the workspace is the only thing that tells
    /// them apart, which is also why the OS window title already carries it.
    ///
    /// Compact drops the label and keeps the icon - at this width the label
    /// has nowhere to go but into the traffic lights - and hands the name to
    /// a tooltip instead, the way a compact agent row does, so a narrow
    /// window still says which workspace it is.
    fn sidebar_title_bar(&self, title: &str, compact: bool, cx: &mut Context<Self>)
                         -> impl IntoElement + use<> {
        let title = title.to_owned();
        TitleBar::new().h(px(window_options::WORKSPACE_TITLE_BAR_HEIGHT))
                       .border_color(gpui_kit::transparent_black())
                       .bg(cx.theme().title_bar)
                       .child(h_flex().id("workspace-title-bar-name")
                                      .flex_1()
                                      .min_w_0()
                                      .gap_2()
                                      .items_center()
                                      .child(div().flex_shrink_0().child(app_titlebar_icon()))
                                      .when(!compact, |row| {
                                          // `min_w_0` as well as `flex_1`: a
                                          // flex child keeps `min-width:
                                          // auto` otherwise, so a long name
                                          // would push the traffic lights
                                          // rather than ellipsize.
                                          row.child(div().flex_1()
                                                         .min_w_0()
                                                         .overflow_hidden()
                                                         .whitespace_nowrap()
                                                         .text_ellipsis()
                                                         .child(title.clone()))
                                      })
                                      .when(compact, |row| {
                                          row.tooltip(move |window, cx| {
                                                 Tooltip::new(title.clone()).build(window, cx)
                                             })
                                      }))
    }

    /// The row under the agent list: the New Agent button and the chevron
    /// that opens the bench beside it. Compact keeps the icon and moves the
    /// label into a tooltip, so the control still says what it does.
    fn new_agent_button(&self, compact: bool, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let hint = self.key_hints.shown().map(|hints| hints.new_agent.clone());
        // Against the row's trailing edge at either width. The compact
        // corner badge would overhang the sidebar's edge from a row this
        // wide, and a compact row centres its icon-only button well clear of
        // that edge anyway.
        with_key_hint(h_flex(), hint.as_deref(), false, cx).flex_shrink_0()
                .h(px(48.))
                .w_full()
                .items_center()
                .px_4()
                .gap_2()
                .when(compact, |row| row.justify_center())
                .border_t_1()
                .border_color(cx.theme().border)
                .child(Button::new("workspace-new-agent").icon(IconName::Plus)
                                                         .when(!compact, |button| {
                                                             button.label(knot_core::l10n::t("sidebar.new_agent"))
                                                         })
                                                         .when(compact, |button| {
                                                             button.tooltip(knot_core::l10n::t("sidebar.new_agent"))
                                                         })
                                                         .ghost()
                                                         .on_click(cx.listener(|view,
                                                                    _: &ClickEvent,
                                                                    _window,
                                                                    cx| {
                                                             view.open_new_agent_dialog(cx);
                                                         })))
                .child(self.bench_popover(cx))
    }

    /// The sidebar column: the window's own title bar, the scrolling agent
    /// list with the dashboard row above it, the error line, and the new
    /// agent button.
    pub(super) fn sidebar_column(&self, title: &str, compact: bool, rows: SidebarRows,
                                 background_targets: SidebarMenuTargets, cx: &mut Context<Self>)
                                 -> impl IntoElement + use<> {
        let SidebarRows { dashboard_row,
                          pull_requests_row,
                          agent_rows, } = rows;
        let row_bounds = Rc::clone(&self.agent_drag.row_bounds);
        // The sidebar column owns the traffic lights (Swift's own
        // sidebar panel does the same - they sit within its width,
        // not the content pane's). The content header below is a
        // plain sibling row, not part of this TitleBar, so it
        // starts at this column's true right edge with no gutter
        // GPUI reserves inside TitleBar for the traffic lights -
        // that's what kept misaligning it with the divider below.
        v_flex().w_full()
                .h_full()
                .bg(cx.theme().title_bar)
                .child(self.sidebar_title_bar(title, compact, cx))
                .child(
                       div().id("workspace-agent-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .child(
            v_flex().min_h_full()
                    .gap_1()
                    .p_4()
                    .child(dashboard_row)
                    .children(pull_requests_row)
                    // Its own container, with no pinned row inside it: row
                    // indices here match `agent_ids` indices exactly, which
                    // is what every gap in `sidebar_drag` is counted
                    // against, and it is what keeps the Dashboard and Pull
                    // Requests rows from ever becoming drop places.
                    .child(v_flex().on_children_prepainted(move |bounds, _window, _cx| {
                                       *row_bounds.borrow_mut() = bounds;
                                   })
                                   .id("workspace-agent-rows")
                                   .gap_1()
                                   .children(agent_rows))
                    // The background menu hangs off a
                    // filler below the rows rather
                    // than off the scroll container:
                    // in GPUI every hitbox under the
                    // pointer counts as hovered, not
                    // just the innermost, so a
                    // container-level context menu
                    // would open on top of the row's
                    // own - which `agent-list-ui`
                    // forbids. A sibling that claims
                    // the leftover space is reached
                    // only by a right-click that
                    // missed every row.
                    .child(
                div().id("workspace-agent-list-background")
                     .flex_1()
                     .min_h(px(32.))
                     // The one drop place no row's own bounds cover:
                     // releasing below the last row moves the dragged
                     // agent to the end of the list (`agent-list-ui`,
                     // "Dragging an agent to the end").
                     .on_drag_move(cx.listener(
                         move |view, event: &gpui_kit::DragMoveEvent<crate::workspace_window::sidebar_drag::AgentDrag>, _window, cx| {
                             let end = view.agent_drag.rows.len();
                             if !event.bounds.contains(&event.event.position) {
                                 if view.agent_drag.target.is_some_and(|target| target.row == end) {
                                     view.agent_drag.target = None;
                                     cx.notify();
                                 }
                                 return;
                             }
                             let target = Some(crate::workspace_window::sidebar_drag::DropTarget { row: end,
                                                                                 gap: end });
                             if view.agent_drag.target != target {
                                 view.agent_drag.target = target;
                                 cx.notify();
                             }
                         },
                     ))
                     .on_drop(cx.listener(
                         move |view, drag: &crate::workspace_window::sidebar_drag::AgentDrag, _window, cx| {
                             view.agent_dropped_at_end(drag, cx);
                         },
                     ))
                     .context_menu(move |menu, window, cx| {
                         sidebar_background_context_menu(&background_targets, menu, window, cx)
                     }),
            ),
        ),
        )
                .children(self.error
                              .as_ref()
                              .map(|error| div().text_sm().px_4().child(error.clone())))
                .child(self.new_agent_button(compact, cx))
    }
}

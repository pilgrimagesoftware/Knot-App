//! The layout area: the agent panes the content column shows beside the
//! side panels, and what each pane draws (`workspace-layouts`, "Pane
//! contents").
//!
//! In single mode the area is exactly the one pane it has always been, so a
//! window that has never been split draws the same tree as before layouts.
//! In a multi-pane layout the panes sit in a `relative()` box, each absolutely
//! positioned from `layout::pane_rects`, and the dividers (`divider`) are laid
//! over them by the window (`workspace_window::layout`).
//!
//! The panes are placed with fractions of the box rather than pixels. The
//! geometry is the same pure function the dividers use, asked for a one-pixel
//! square, so a pane and the divider beside it cannot disagree about where the
//! boundary is, and nothing has to measure the box before it can be drawn.
//!
//! Not here: a pane's frame (`pane_frame`), the divider element and its drag,
//! and the side panels.

use gpui_kit::AnyElement;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::size;
use knot_core::LayoutMode;
use knot_core::WorkspaceUiState;
use uuid::Uuid;

use super::layout::pane_rects;
use super::pane_frame::PaneSlot;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The layout area with the side panels beside it: the agent panes, the
    /// dividers over them, then whichever of the git and artifact panels are
    /// open, which follow the focused agent.
    pub(super) fn layout_area(&mut self, window: &mut Window, cx: &mut Context<Self>)
                              -> AnyElement {
        // The read `prepare_frame` synced the selection to, not a second one.
        let ui = self.take_frame_layout();
        let panes = if ui.layout_mode == LayoutMode::Single {
            self.single_pane(window, cx)
        }
        else {
            self.layout_panes(&ui, window, cx)
        };
        // Beside the side panels' group, not inside it: the dividers belong
        // to the layout area. Single mode returns `panes` as is.
        let panes = self.with_layout_dividers(panes, cx);
        let panes = self.with_side_panels(panes, window, cx);
        // A repository waiting to be trusted is the window's question, not
        // one agent's, so it sits over every pane.
        let trust_banner = self.trust_banner(cx);
        v_flex().flex_1()
                .min_h_0()
                .w_full()
                .children(trust_banner)
                .child(panes)
                .into_any_element()
    }

    /// The one pane a single layout shows: the selected agent's, with the
    /// Codex usage banner above it and its activity noted (#404).
    fn single_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let pane = match self.selected_agent {
            Some(id) => self.render_agent_pane(id, window, cx),
            None => self.empty_pane("Choose an agent from the sidebar", cx),
        };
        // Codex nearing its usage limit (#291).
        let pane = self.with_usage_banner(pane, cx);
        // Typing, clicking or scrolling in the pane restarts the agent's idle
        // period (#404).
        match self.selected_agent {
            Some(id) => self.with_activity_capture(id, pane, cx),
            None => pane,
        }
    }

    /// The panes of a multi-pane layout, each in its rectangle of the area.
    fn layout_panes(&mut self, ui: &WorkspaceUiState, window: &mut Window,
                    cx: &mut Context<Self>)
                    -> AnyElement {
        let secondary = ui.split_ratio_secondary
                          .unwrap_or(knot_core::consts::WORKSPACE_SPLIT_RATIO_DEFAULT);
        // A one-pixel square, so the rectangles come out as fractions of the
        // area; see the module doc.
        let rects = pane_rects(ui.layout_mode,
                               ui.split_ratio,
                               secondary,
                               size(px(1.), px(1.)));
        let focus = usize::try_from(ui.focused_pane_index).unwrap_or(0);
        // `ui` is the read `sync_selection_with_layout` set the selection
        // from, so the two agree unless something else wrote the selection
        // since - a second writer.
        if let Some(focused) = super::super::layout_select::focused_agent(ui) {
            debug_assert_eq!(self.selected_agent,
                             Some(focused),
                             "selected_agent must be the focused pane's agent");
        }
        let mut area = div().id("layout-area")
                            .debug_selector(|| "layout-area".to_string())
                            .size_full()
                            .relative();
        for (index, (id, rect)) in ui.active_agent_ids.iter().copied().zip(rects).enumerate() {
            let content = self.render_agent_pane(id, window, cx);
            // The banner speaks for the selected agent, so one pane carries
            // it, not every pane.
            let content = if self.selected_agent == Some(id) {
                self.with_usage_banner(content, cx)
            }
            else {
                content
            };
            let content = self.with_activity_capture(id, content, cx);
            area = area.child(self.pane_frame(&PaneSlot { index,
                                                          agent: id,
                                                          rect,
                                                          focused: index == focus },
                                              content,
                                              cx));
        }
        area.into_any_element()
    }

    /// What agent `id`'s pane shows: the stopped view, its conversation panel
    /// or its terminal. One branch for every pane of every layout, so a pane
    /// in a split shows what the agent shows when it is the only one.
    pub(super) fn render_agent_pane(&mut self, id: Uuid, window: &mut Window,
                                    cx: &mut Context<Self>)
                                    -> AnyElement {
        // The markdown file and the diagram are no longer read here. They
        // used to take the content area ahead of either session pane; now
        // they are sections of the artifact panel, which `with_side_panels`
        // puts beside the panes.
        let (is_panel_mode, stopped) = {
            let store = self.store.lock();
            let agent = store.agent(id);
            (agent.map(|agent| agent.view_mode) == Some(knot_core::ViewMode::Panel),
             agent.filter(|agent| !agent.activated)
                  .map(|agent| agent.name.clone()))
        };
        // Ahead of both session panes, which would otherwise render empty:
        // there is no session, and asking for one is what the activation gate
        // refuses.
        if let Some(name) = stopped {
            return self.render_stopped_pane(name, cx);
        }
        if is_panel_mode {
            return self.render_panel_pane(id, window, cx);
        }
        // The view owns its input, sizing and repaint; see `terminal_pane`.
        match self.terminal_surface(id) {
            Some(surface) => surface,
            None => self.empty_pane("Starting terminal…", cx),
        }
    }

    /// A pane with nothing to show yet, and a line saying why.
    fn empty_pane(&self, text: &'static str, cx: &Context<Self>) -> AnyElement {
        div().size_full()
             .p_6()
             .flex()
             .items_center()
             .justify_center()
             .bg(cx.theme().muted)
             .child(div().text_color(cx.theme().muted_foreground).child(text))
             .into_any_element()
    }
}

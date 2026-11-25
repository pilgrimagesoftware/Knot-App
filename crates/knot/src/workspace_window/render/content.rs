//! The content column: everything to the right of the sidebar - the
//! selected agent's header strip and the pane below it (terminal, panel,
//! markdown viewer, or the dashboard).

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::resizable::ResizableState;
use gpui_kit::component::resizable::h_resizable;
use gpui_kit::component::resizable::resizable_panel;
use gpui_kit::px;

use crate::consts;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The column that fills the window beside the sidebar.
    ///
    /// `min_w_0` throughout: a wide panel message (a markdown table, a long
    /// command line) must wrap inside this column rather than stretch it
    /// past the window and push the composer's Send button off screen - see
    /// `.claude/rules/knot-ui-conventions.md`, "Flex overflow".
    pub(super) fn content_column(&mut self, is_dashboard: bool,
                                 dashboard_content: Option<gpui_kit::AnyElement>,
                                 title_bar_left: gpui_kit::AnyElement,
                                 title_bar_right: gpui_kit::AnyElement, window: &mut Window,
                                 cx: &mut Context<Self>)
                                 -> gpui_kit::AnyElement {
        // `min_w_0` so a wide panel message (a markdown table, a
        // long command line) wraps inside this column instead of
        // stretching it past the window and pushing the prompt
        // input's Send button off screen - see
        // `knot-ui-conventions.md`'s "Flex overflow" rule.
        v_flex().flex_1()
                .min_w_0()
                .h_full()
                .children((!is_dashboard).then(|| {
                                             // `min_w_0` on the row and a
                                             // non-shrinking right
                                             // side: at a narrow window the
                                             // agent's status line
                                             // used to push the whole header
                                             // wider than the
                                             // pane, clipping the title on one
                                             // edge and running
                                             // the diff stat off the other.
                                             h_flex().w_full()
                                                     .min_w_0()
                                                     .flex_shrink_0()
                                                     .h(px(64.))
                                                     .items_center()
                                                     .justify_between()
                                                     .gap_3()
                                                     .px_5()
                                                     .bg(cx.theme().background)
                                                     // The rule between the
                                                     // header and the pane
                                                     // (#578), drawn like the
                                                     // window's other
                                                     // separators. On the
                                                     // header row, so it runs
                                                     // the content column's
                                                     // full width, from the
                                                     // sidebar to the edge.
                                                     .border_b_1()
                                                     .border_color(cx.theme().border)
                                                     .child(title_bar_left)
                                                     .child(h_flex().flex_shrink_0()
                                                                    .items_center()
                                                                    .gap_2()
                                                                    .child(title_bar_right))
                                         }))
                .child(dashboard_content.unwrap_or_else(|| {
                                            // `flex_1().min_h_0()`, not
                                            // `size_full()`: this box
                                            // is a sibling of the 64px title
                                            // bar above it, so a
                                            // full height makes it overflow its
                                            // container by
                                            // exactly that much and pushes the
                                            // input area's
                                            // control row and Send button below
                                            // the window edge.
                                            v_flex().flex_1()
                                                    .min_h_0()
                                                    .w_full()
                                                    // The pane goes in a `flex_1().min_h_0()` box
                                                    // of its own
                                                    // because the processes section is its sibling
                                                    // below:
                                                    // every pane inside styles itself `size_full`,
                                                    // which
                                                    // would otherwise overflow this column by
                                                    // exactly the
                                                    // section's height.
                                                    // The pane is built first and handed to
                                                    // `with_side_panels`, which puts the side
                                                    // panels
                                                    // beside it
                                                    // when that panel is open. Two statements
                                                    // rather than
                                                    // one expression: both borrow `self` mutably,
                                                    // so they
                                                    // cannot be nested in a single call.
                                                    // The layout area: the agent panes and the side
                                                    // panels beside them. See `panes`.
                                                    .child(self.layout_area(window, cx))
                                                    // Below whichever session pane is showing, so
                                                    // the
                                                    // terminal and panel views get the sections
                                                    // from
                                                    // one place rather than two that can drift.
                                                    .children(self.agent_sections_row(cx))
                                                    .into_any_element()
                                        }))
                .into_any_element()
    }

    /// The selected agent's pane with whichever side panels are open beside
    /// it: the git panel, the artifact panel, or both.
    ///
    /// Beside rather than over: the Swift panels are siblings in an `HStack`,
    /// so the content narrows rather than being occluded, and the agent stays
    /// visible while its work is reviewed - which is the point of reviewing
    /// it here rather than in another window.
    ///
    /// One resizable group holds all of them rather than one group nested in
    /// another. One group means one drag model: each handle moves the
    /// boundary it sits on and the group reconciles the rest. Nesting would
    /// make the outer drag resize a subtree containing a panel with its own
    /// fixed width, and that panel would absorb or refuse the change
    /// depending on which side was dragged.
    ///
    /// The exception is an expanded artifact panel, which takes the content
    /// area outright: no group, no handles, and the content pane is not
    /// drawn. That is the one state `acp-panel-ui` and `terminal-input`
    /// withhold focus for.
    pub(super) fn with_side_panels(&mut self, pane: gpui_kit::AnyElement, window: &mut Window,
                                   cx: &mut Context<Self>)
                                   -> gpui_kit::AnyElement {
        let Some(id) = self.selected_agent
        else {
            return pane;
        };
        let snapshot = self.artifact_snapshot(id);
        let artifact = self.render_artifact_panel(id, &snapshot, window, cx);

        // Expanded: the panel is the content area. The sidebar and any open
        // git panel are outside this element and so are unaffected, which is
        // what `ArtifactPanelView` achieves by giving the terminal area
        // `width: 0` and `opacity: 0` rather than by removing it.
        if let Some(panel) = artifact.as_ref()
                                     .filter(|_| self.artifact_panel_expanded(id))
        {
            let _ = panel;
            return artifact.expect("matched just above");
        }

        let git = self.agent_folder(id)
                      .and_then(|folder| self.git_panel_pane(id, &folder, window, cx));
        if git.is_none() && artifact.is_none() {
            return pane;
        }

        // A resizable group rather than a hand-rolled drag handle: the
        // sidebar divider already works this way, and the group carries the
        // clamp, so a panel cannot be dragged past the spec's bounds.
        let state = self.git
                        .git_panel_resize
                        .entry(id)
                        .or_insert_with(|| cx.new(|_| ResizableState::default()))
                        .clone();
        let git_width = self.git_panel_width(id);
        let artifact_width = self.artifact_arrangement(id).width;
        let has_git = git.is_some();

        let group = h_resizable("side-panel-split").with_state(&state)
            .on_resize(cx.listener(move |view, state: &Entity<ResizableState>, _window, cx| {
                          let sizes = state.read(cx).sizes();
                          // The panels are the trailing entries, in the order
                          // they were added: git then artifact. Read from the
                          // end so the content pane's own size is skipped.
                          let mut trailing = sizes.iter().rev();
                          if let Some(width) =
                              trailing.next().filter(|_| view.artifact_panel_open(id))
                          {
                              view.set_artifact_panel_width(id, f32::from(*width));
                          }
                          if let Some(width) = trailing.next().filter(|_| has_git) {
                              view.set_git_panel_width(id, f32::from(*width), cx);
                          }
                      }))
            // The content pane keeps a floor of its own: two panels at their
            // own minimum would otherwise leave the conversation nothing on a
            // narrow window. Expand is the deliberate case and is handled
            // above, before this group is built.
            .child(resizable_panel().size_range(px(crate::consts::CONTENT_PANE_MIN_WIDTH)
                                                ..px(f32::MAX))
                                    .child(v_flex().flex_1().min_w_0().h_full().child(pane)));
        let group = if let Some(panel) = git {
            group.child(resizable_panel().size(px(git_width))
                                         .size_range(px(consts::GIT_PANEL_MIN_WIDTH)
                                                     ..px(consts::GIT_PANEL_MAX_WIDTH))
                                         // A sized panel beside a flexible
                                         // one has to opt out of growing, or
                                         // it takes the slack back on the
                                         // frame after a drag.
                                         .flex_none()
                                         .child(panel))
        }
        else {
            group
        };
        let group = if let Some(panel) = artifact {
            group.child(resizable_panel().size(px(artifact_width))
                                         .size_range(px(consts::ARTIFACT_PANEL_MIN_WIDTH)
                                                     ..px(consts::ARTIFACT_PANEL_MAX_WIDTH))
                                         .flex_none()
                                         .child(panel))
        }
        else {
            group
        };
        group.into_any_element()
    }
}

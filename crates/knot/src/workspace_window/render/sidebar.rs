//! The sidebar's agent rows: one element per agent, with the selection, drag,
//! click and context-menu wiring. The expanded body is in `sidebar_row`, the
//! dashboard row above the list in `dashboard_row`.
//!
//! An agent row is built from an [`AgentRow`] snapshot rather than from the
//! store directly, so the store lock is released before any element is
//! built - see [`super`]'s `render`.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::AppContext;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::DragMoveEvent;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use unicode_segmentation::UnicodeSegmentation;

use super::sidebar_compact::{
    CompactAgentRow, CompactTooltip, compact_agent_row_body, compact_tooltip,
};
use super::sidebar_row::ExpandedAgentRow;
use super::sidebar_row::expanded_agent_row_body;
use crate::consts;
use crate::workspace_window::AgentMenuTargets;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::agent_row::AgentRow;
use crate::workspace_window::agent_row_context_menu;
use crate::workspace_window::key_hints::with_key_hint;
use crate::workspace_window::sidebar_drag::AgentDrag;
use crate::workspace_window::sidebar_drag::AgentDragPreview;
use crate::workspace_window::sidebar_drag::AgentRowFlags;
use crate::workspace_window::sidebar_drag::DropEdge;
use crate::workspace_window::sidebar_drag::drop_line;
use crate::workspace_window::sidebar_drag::drop_line_edge;

impl WorkspaceWindow {
    /// One element per agent in this workspace, in sidebar order.
    ///
    /// Collected rather than returned lazily because the closures borrow
    /// `cx`, which the caller needs back to build the rest of the tree.
    /// `compact` swaps each row's *body* for the avatar-only one; everything
    /// around the body - selection, dimming, the companion indent, the click
    /// handler and the context menu - is wired once and applies either way.
    pub(super) fn agent_rows(&mut self, agents: Vec<AgentRow>, title_font_name: String,
                             title_font_size: gpui_kit::Pixels, compact: bool,
                             cx: &mut Context<Self>)
                             -> Vec<gpui_kit::AnyElement> {
        // Rebuilt every frame, read only during a drag: the group boundaries
        // `drop_gap` resolves a hovered row's edges through.
        self.agent_drag.rows =
            agents.iter()
                  .map(|agent| AgentRowFlags { is_companion: agent.is_companion, })
                  .collect();
        let row_count = agents.len();
        let drag_target_gap = self.agent_drag.target.map(|target| target.gap);
        let row_bounds = Rc::clone(&self.agent_drag.row_bounds);
        let window_weak = cx.entity().downgrade();

        let store_for_menu = Arc::clone(&self.store);
        let workspace_id = self.workspace_id;
        let window_entity = cx.entity();

        let hints = self.key_hints.shown().cloned();
        agents.into_iter()
              .enumerate()
              .map(|(index,
                     AgentRow { id,
                                avatar,
                                name,
                                folder,
                                state,
                                idle_since,
                                input_reason,
                                session_limit,
                                is_shell,
                                is_companion,
                                is_locked,
                                autopilot,
                                header_title,
                                persona_name,
                                agent_type,
                                is_running,
                                schedule_line,
                                idle_line, })| {
                       let hint = hints.as_ref()
                                       .and_then(|hints| hints.agent(index))
                                       .map(str::to_owned);
                       let menu_name = name.clone();
                       let tooltip = CompactTooltip { name:          name.clone(),
                                                      state,
                                                      idle_since,
                                                      input_reason: input_reason.clone(),
                                                      session_limit: session_limit.clone(),
                                                      is_shell,
                                                      status:        header_title.clone(),
                                                      scheduling:    schedule_line.iter()
                                                                                  .chain(&idle_line)
                                                                                  .cloned()
                                                                                  .collect(), };
                       let preview_name = name.clone();
                       let menu_folder = folder.clone();
                       let folder_name =
                           PathBuf::from(&folder).file_name()
                                                 .map(|name| name.to_string_lossy().into_owned())
                                                 .unwrap_or(folder);
                       // Legacy/imported data may carry more
                       // than one character;
                       // clamp to a single grapheme so it can't
                       // overflow the tile.
                       let avatar = avatar.graphemes(true)
                                          .next()
                                          .unwrap_or(consts::DEFAULT_AGENT_AVATAR)
                                          .to_string();
                       let preview_avatar = avatar.clone();
                       let drop_line_at =
                           drag_target_gap.and_then(|gap| drop_line_edge(index, row_count, gap));
                       let selected = self.selected_agent == Some(id);
                       // A plain clickable div, not `Button` -
                       // `Button`'s default
                       // sizing forces a fixed height
                       // regardless of content,
                       // clipping this row's up to 4 lines
                       // (name/persona/status/
                       // folder). Same fix as the dashboard
                       // cards in
                       // `dashboard.rs`.
                       div().id(gpui_kit::ElementId::from(format!("workspace-agent-{id}")))
                            .debug_selector(|| format!("workspace-agent-row-{index}"))
                            .relative()
                            .on_drag_move(cx.listener(move |view,
                                                       event: &DragMoveEvent<AgentDrag>,
                                                       _window,
                                                       cx| {
                                              view.agent_drag_moved(index, event, cx);
                                          }))
                            .on_drop(cx.listener(move |view, drag: &AgentDrag, _window, cx| {
                                          view.agent_dropped(drag, index, cx);
                                      }))
                            // Only the whole row starts a drag - no separate
                            // handle, since an agent row carries no buttons or
                            // text fields a handle would need to spare. A
                            // companion does not drag on its own
                            // (`agent-list-ui`); its owner's row does, and the
                            // companion moves with it.
                            .when(!is_companion, |row| {
                                let window_weak = window_weak.clone();
                                let row_bounds = Rc::clone(&row_bounds);
                                row.on_drag(
                                    AgentDrag { id, row: index },
                                    move |_drag, cursor_offset, window, cx| {
                                        // A target left over from a drag
                                        // released outside every row must not
                                        // draw during this one.
                                        window_weak.update(cx, |view, _| {
                                                        view.agent_drag.target = None;
                                                    })
                                                   .ok();
                                        let press = window.mouse_position() - cursor_offset;
                                        let bounds = row_bounds.borrow().get(index).copied();
                                        let preview = AgentDragPreview::new(preview_name.clone(),
                                                                            preview_avatar.clone(),
                                                                            bounds,
                                                                            press);
                                        cx.new(|_| preview)
                                    },
                                )
                            })
                            // Hidden once the drag ends, whether or not it
                            // ended in a drop: a release outside every row
                            // leaves the target set.
                            .children(drop_line_at.filter(|_| cx.has_active_drag())
                                                  .map(|edge: DropEdge| drop_line(edge, cx)))
                            .cursor_pointer()
                            .rounded(cx.theme().radius)
                            .p_2()
                            // A companion belongs to the agent above it, so it reads
                            // as nested: indented, with a rule down its left edge.
                            .when(is_companion, |row| {
                                row.ml_4().border_l_2().border_color(cx.theme().border)
                            })
                            .bg(if selected {
                                cx.theme().muted
                            }
                            else {
                                cx.theme().transparent
                            })
                            // A stopped agent's row is dimmed as a whole, so a
                            // workspace of mixed agents reads at a glance. The
                            // state dot cannot carry this: its four values say what
                            // a *running* agent is doing, and none of them means
                            // "not running at all". Selection still highlights the
                            // row underneath, so the selected-but-stopped agent
                            // whose pane shows the stopped placeholder is still
                            // visibly the selected one.
                            .when(!is_running, |row| row.opacity(0.45))
                            // The compact row hides the name and the status lines,
                            // so they become the row's tooltip - otherwise an avatar
                            // is the only thing left to tell two agents apart, and
                            // what each is doing is out of sight (#253).
                            .when(compact, |row| {
                                row.tooltip(move |window, cx| {
                                       Tooltip::new(compact_tooltip(&tooltip)).build(window, cx)
                                   })
                            })
                            .map(|row| {
                                if compact {
                                    row
                                }
                                else {
                                    with_key_hint(row, hint.as_deref(), false, cx)
                                }
                            })
                            .child(if compact {
                                compact_agent_row_body(CompactAgentRow { id,
                                                                         avatar,
                                                                         state,
                                                                         idle_since,
                                                                         input_reason: input_reason.clone(),
                                                                         session_limit: session_limit.clone(),
                                                                         is_shell,
                                                                         hint: hint.clone() },
                                                       cx).into_any_element()
                            }
                            else {
                                expanded_agent_row_body(ExpandedAgentRow { id,
                                                                           avatar,
                                                                           name,
                                                                           folder_name,
                                                                           state,
                                                                           idle_since,
                                                                           input_reason: input_reason.clone(),
                                                                           session_limit: session_limit.clone(),
                                                                           is_shell,
                                                                           is_companion,
                                                                           is_locked,
                                                                           autopilot,
                                                                           header_title,
                                                                           persona_name,
                                                                           agent_type,
                                                                           schedule_line: schedule_line.clone(),
                                                                           idle_line: idle_line.clone() },
                                                        &title_font_name,
                                                        title_font_size,
                                                        cx)
                            })
                            .on_click(cx.listener(move |view, event: &ClickEvent, _window, cx| {
                                            // The platform's secondary modifier
                                            // (⌘ on macOS, Ctrl elsewhere),
                                            // which is what the rest of the
                                            // app's shortcuts read, so ⌘-click
                                            // on a Mac and Ctrl-click on
                                            // Linux skip the companion layout.
                                            view.select_agent(id,
                                                              event.modifiers().secondary(),
                                                              cx);
                                            // Leave the dashboard, the same way
                                            // tapping an
                                            // agent card does - selecting a row
                                            // while the
                                            // dashboard was open used to change
                                            // the selection
                                            // without ever showing the session.
                                            view.view_mode = WorkspaceViewMode::Terminal;
                                            cx.notify();
                                        }))
                            .context_menu({
                                let targets =
                                    AgentMenuTargets { store: Arc::clone(&store_for_menu),
                                                       window_entity: window_entity.clone(),
                                                       workspace_id,
                                                       id,
                                                       name: menu_name.clone(),
                                                       folder: menu_folder.clone() };
                                move |menu, window, cx| {
                                    agent_row_context_menu(&targets, menu, window, cx)
                                }
                            })
                   })
              .map(gpui_kit::IntoElement::into_any_element)
              .collect()
    }
}

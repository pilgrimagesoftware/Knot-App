//! The finder overlay: a dimmed backdrop over the window, and a panel with
//! the query field and its results, as the reference drew it.

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::MouseButton;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::IconName;
use gpui_kit::component::input::Enter;
use gpui_kit::component::input::Escape;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::MoveDown;
use gpui_kit::component::input::MoveUp;
use gpui_kit::div;
use gpui_kit::px;

use super::state::FinderRows;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::marked::marked_text;

/// The panel's width and the results' height cap: the reference's.
const PANEL_WIDTH: f32 = 650.;
const RESULTS_MAX_HEIGHT: f32 = 450.;

impl WorkspaceWindow {
    /// The overlay, when the finder is open.
    pub(in crate::workspace_window) fn render_file_finder(&mut self, cx: &mut Context<Self>)
                                                          -> Option<impl IntoElement + use<>> {
        let rows = self.file_finder_rows(cx);
        let finder = self.file_finder.as_ref()?;
        let selected = finder.selected;
        let input = finder.input.clone();
        let entity = cx.entity();
        let count = match &rows {
            FinderRows::Results(rows) => rows.len(),
            _ => 0,
        };

        let body = match &rows {
            FinderRows::Empty => None,
            FinderRows::Results(rows) => {
                let accent = cx.theme().accent;
                let list = v_flex().py_1().children(rows.iter().enumerate().map(|(index, row)| {
                    let entity = entity.clone();
                    div().id(("file-finder-row", index))
                         .px_3()
                         .py_1()
                         .when_selected_row(index == selected, cx)
                         .font_family(cx.theme().mono_font_family.clone())
                         .text_sm()
                         .overflow_hidden()
                         .whitespace_nowrap()
                         .text_ellipsis()
                         .child(marked_text(row.entry.token.clone(), &row.matched, 0, accent))
                         .on_click(move |_, window, app| {
                             entity.update(app, |view, cx| {
                                       view.choose_file_finder(Some(index), window, cx)
                                   });
                         })
                }));
                Some(crate::capped_scroll::capped_scroll_list("file-finder-results",
                                                              px(RESULTS_MAX_HEIGHT),
                                                              list).into_any_element())
            }
            status => {
                let key = match status {
                    FinderRows::Gathering => "finder.gathering",
                    FinderRows::TooManyFiles => "finder.too_many",
                    FinderRows::Matching => "finder.matching",
                    _ => "finder.no_matches",
                };
                Some(div().py_6()
                          .w_full()
                          .flex()
                          .justify_center()
                          .text_color(cx.theme().muted_foreground)
                          .child(knot_core::l10n::t(key))
                          .into_any_element())
            }
        };

        let panel =
            v_flex().w(px(PANEL_WIDTH))
                    .max_w_full()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(12.))
                    .shadow_lg()
                    .overflow_hidden()
                    // Clicks inside the panel are the panel's; only the
                    // backdrop dismisses.
                    .on_mouse_down(MouseButton::Left, |_, _, app| app.stop_propagation())
                    .capture_action::<MoveUp>({
                        let entity = entity.clone();
                        move |_, _, app| {
                            entity.update(app, |view, cx| {
                                      view.move_file_finder(-1, count);
                                      cx.stop_propagation();
                                      cx.notify();
                                  });
                        }
                    })
                    .capture_action::<MoveDown>({
                        let entity = entity.clone();
                        move |_, _, app| {
                            entity.update(app, |view, cx| {
                                      view.move_file_finder(1, count);
                                      cx.stop_propagation();
                                      cx.notify();
                                  });
                        }
                    })
                    .capture_action::<Enter>({
                        let entity = entity.clone();
                        move |_, window, app| {
                            entity.update(app, |view, cx| {
                                      view.choose_file_finder(None, window, cx);
                                      cx.stop_propagation();
                                  });
                        }
                    })
                    .capture_action::<Escape>({
                        let entity = entity.clone();
                        move |_, window, app| {
                            entity.update(app, |view, cx| {
                                      view.close_file_finder(window, cx);
                                      cx.stop_propagation();
                                  });
                        }
                    })
                    .child(h_flex().gap_2()
                                   .px_4()
                                   .py_3()
                                   .child(Icon::new(IconName::Search).text_color(cx.theme()
                                                                                   .muted_foreground))
                                   .child(div().flex_1().min_w_0().child(Input::new(&input))))
                    .children(body.map(|body| {
                                      v_flex().border_t_1()
                                              .border_color(cx.theme().border)
                                              .child(body)
                                  }));

        Some(div().absolute()
                  .inset_0()
                  .flex()
                  .justify_center()
                  .pt(px(80.))
                  .bg(gpui_kit::black().opacity(0.4))
                  .on_mouse_down(MouseButton::Left, move |_, window, app| {
                      entity.update(app, |view, cx| view.close_file_finder(window, cx));
                  })
                  .child(panel))
    }
}

/// Tints the selected result.
trait SelectedRow {
    fn when_selected_row(self, selected: bool, cx: &Context<WorkspaceWindow>) -> Self;
}

impl SelectedRow for gpui_kit::Stateful<gpui_kit::Div> {
    fn when_selected_row(self, selected: bool, cx: &Context<WorkspaceWindow>) -> Self {
        if selected {
            self.bg(cx.theme().accent)
                .text_color(cx.theme().accent_foreground)
        }
        else {
            self
        }
    }
}

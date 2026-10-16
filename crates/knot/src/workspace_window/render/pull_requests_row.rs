//! The Pull Requests launcher row in the sidebar.
//!
//! Shaped like the dashboard row above it, for the same reason that one is
//! not an icon in the bottom bar: it is a view of the whole workspace, so it
//! belongs with the rows it summarises.
//!
//! What it adds is the count (#210): how many pull requests, open issues and
//! OpenSpec changes the Changes view holds, worded by
//! `changes_counts::ChangesCounts::summary`. The pull requests' own states -
//! open, merged, closed - are in its tooltip, so the line stays short enough
//! for a sidebar.

use gpui_kit::base::{StyledExt, h_flex};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};

use crate::pull_request_state::PullRequestCounts;
use crate::workspace_window::key_hints::with_key_hint;
use crate::workspace_window::{WorkspaceViewMode, WorkspaceWindow};

impl WorkspaceWindow {
    /// The Changes row, which opens the Changes view (#504).
    ///
    /// Always present: the view holds the repositories' issues and OpenSpec
    /// changes as well as pull requests, so a workspace with none of them
    /// still has somewhere for the row to lead.
    pub(super) fn pull_requests_row(&self, compact: bool, cx: &mut Context<Self>)
                                    -> gpui_kit::AnyElement {
        let summary = self.changes_counts().summary();
        let pull_requests = self.pull_request_counts();
        let is_showing = self.view_mode == WorkspaceViewMode::Changes;

        // The second line ellipsizes in a narrow sidebar, and the compact
        // sidebar drops it altogether - so the whole of it goes in a tooltip,
        // with the pull requests' states, the one place both are always
        // readable.
        let tooltip = row_tooltip(summary.as_deref(),
                                  (pull_requests.total() > 0).then(|| counts_label(pull_requests)));

        let hint = self.key_hints
                       .shown()
                       .map(|hints| hints.pull_requests.clone());
        let row = div().id("workspace-pull-requests-row");
        let row = if compact {
            row
        }
        else {
            with_key_hint(row, hint.as_deref(), false, cx)
        };
        row.cursor_pointer()
           .rounded(cx.theme().radius)
           .p_2()
           .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
           .bg(if is_showing {
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
                                       .child(Icon::default().path("icons/git-pull-request.svg"));
                              if compact {
                                  with_key_hint(icon, hint.as_deref(), true, cx)
                              }
                              else {
                                  icon
                              }
                          })
                          .when(!compact, |row| {
                              // `min_w_0` on the growing child, not
                              // just `flex_1`: the breakdown is the
                              // longest text in the sidebar, and
                              // without it the row pushes the divider
                              // instead of ellipsizing.
                              row.child(div().flex_1()
                                             .min_w_0()
                                             .child(title_and_counts(summary.clone(), cx)))
                          }))
           .on_click(cx.listener(|view, _: &ClickEvent, _window, cx| {
                           view.view_mode = view.view_mode.toggled(WorkspaceViewMode::Changes);
                           cx.notify();
                       }))
           .into_any_element()
    }
}

/// The row's label and, under it, what the view holds - or the label alone
/// when it holds nothing Knot has counted.
fn title_and_counts(summary: Option<String>, cx: &mut Context<WorkspaceWindow>)
                    -> gpui_kit::AnyElement {
    gpui_kit::base::v_flex().min_w_0()
                            .child(div().font_semibold()
                                        .child(knot_core::l10n::t("changes_view.title")))
                            .children(summary.map(|summary| {
                                                 div().text_xs()
                                                      .overflow_hidden()
                                                      .whitespace_nowrap()
                                                      .text_ellipsis()
                                                      .text_color(cx.theme().muted_foreground)
                                                      .child(summary)
                                             }))
                            .into_any_element()
}

/// The tooltip: the title with the summary, and the pull requests' states
/// on a line of their own when there are any.
pub(crate) fn row_tooltip(summary: Option<&str>, pull_request_states: Option<String>) -> String {
    let title = knot_core::l10n::t("changes_view.title");
    let mut tooltip = match summary {
        Some(summary) => format!("{title} — {summary}"),
        None => title,
    };
    if let Some(states) = pull_request_states {
        tooltip.push('\n');
        tooltip.push_str(&knot_core::l10n::t_with("changes_view.pull_request_states",
                                                  &[("states", &states)]));
    }
    tooltip
}

/// What the row's tooltip says about the pull requests' states.
///
/// A breakdown once anything has been fetched, and the plain total before
/// that. Showing "0 open · 0 merged · 0 closed" for four records Knot has not
/// asked about yet would state three things it does not know; "4" states the
/// one thing it does.
///
/// A state nothing is in is left out entirely. "0 closed" is a fact nobody
/// needs and a third of the row's width, and dropping it makes the counts
/// that are there easier to read.
///
/// Assembled from per-count entries rather than one sentence, which the
/// localization rule would normally forbid: this is a list of labelled
/// numbers whose membership varies, not a sentence with a value in it, so
/// each part stays independently translatable and the separator is
/// punctuation.
pub(crate) fn counts_label(counts: PullRequestCounts) -> String {
    if counts.total() == 0 {
        return knot_core::l10n::t("changes_view.no_pull_requests");
    }
    if counts.nothing_known() {
        return knot_core::l10n::t_with("pull_requests.counts_total",
                                       &[("count", &counts.total().to_string())]);
    }
    [("pull_requests.count_open", counts.open),
     ("pull_requests.count_merged", counts.merged),
     ("pull_requests.count_closed", counts.closed),
     ("pull_requests.count_pending", counts.pending),
     ("pull_requests.count_not_found", counts.not_found)].into_iter()
                                                         .filter(|(_, count)| *count > 0)
                                                         .map(|(key, count)| {
                                                             let count = count.to_string();
                                                             knot_core::l10n::t_with(key,
                                                                                     &[("count",
                                                                                        &count)])
                                                         })
                                                         .collect::<Vec<_>>()
                                                         .join(" · ")
}

#[cfg(test)]
mod tests;

//! What the report window draws: the form over [`BugReport`] and the
//! Cancel / Report row beneath it. Reads the entity only; every change goes
//! back through it.

use gpui_kit::assets::IconName;
use gpui_kit::base::{Disableable, StyledExt, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, Textarea};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement, SharedString, Styled,
    div,
};

use super::kind::IssueKind;
use super::logs::LogKind;
use super::report::BugReport;
use super::screenshots;
use crate::consts::BUG_REPORT_MAX_SCREENSHOTS;

pub(super) fn report_content(entity: &Entity<BugReport>, app: &App) -> impl IntoElement {
    let report = entity.read(app);
    let theme = app.theme();
    let label = |key: &str| {
        div().text_sm()
             .font_semibold()
             .child(knot_core::l10n::t(key))
    };

    v_flex().gap_2()
            .child(label("bug_report.kind_label"))
            .child(kind_selector(entity, report.kind))
            .child(label("bug_report.subject_label"))
            .child(Input::new(&report.subject))
            .child(label("bug_report.description_label"))
            .child(Textarea::new(&report.description).w_full())
            .child(label("bug_report.diagnostics_label"))
            .child(div().text_xs()
                        .text_color(theme.muted_foreground)
                        .child(knot_core::l10n::t("bug_report.diagnostics_hint")))
            // Read-only rather than disabled: it keeps its normal look and
            // can still be focused, selected and copied.
            .child(Textarea::new(&report.diagnostics).readonly(true)
                                                     .w_full()
                                                     .text_xs())
            .when(report.kind.offers_logs(), |content| {
                content.child(label("bug_report.logs.label"))
                       .child(div().text_xs()
                                   .text_color(theme.muted_foreground)
                                   .child(knot_core::l10n::t("bug_report.logs.hint")))
                       .child(h_flex().gap_4()
                                      .children(LogKind::ALL.map(|kind| {
                                                                log_checkbox(entity, report, kind)
                                                            })))
            })
            .child(label("bug_report.screenshots.label"))
            .child(div().text_xs()
                        .text_color(theme.muted_foreground)
                        .child(knot_core::l10n::t_with("bug_report.screenshots.hint",
                                                       &[("max",
                                                          &BUG_REPORT_MAX_SCREENSHOTS.to_string())])))
            .child(screenshots_section(entity, report, app))
            .children(report.phase.message().map(|message| {
                                                div().text_sm()
                                                     .text_color(if report.phase.is_error() {
                                                                     theme.danger
                                                                 }
                                                                 else {
                                                                     theme.muted_foreground
                                                                 })
                                                     .child(message)
                                            }))
}

fn kind_selector(entity: &Entity<BugReport>, selected: IssueKind) -> RadioGroup {
    let entity = entity.clone();
    RadioGroup::horizontal("issue-kind")
        .children(IssueKind::ALL.map(|kind| Radio::new(kind.to_string()).label(kind.label())))
        .selected_index(IssueKind::ALL.iter().position(|kind| *kind == selected))
        .on_click(move |index, window, app| {
            if let Some(&kind) = IssueKind::ALL.get(*index) {
                entity.update(app, |report, cx| report.set_kind(kind, window, cx));
            }
        })
}

fn log_checkbox(entity: &Entity<BugReport>, report: &BugReport, kind: LogKind) -> Checkbox {
    let id = match kind {
        LogKind::App => "attach-app-log",
        LogKind::Mcp => "attach-mcp-log",
    };
    let entity = entity.clone();
    Checkbox::new(id).label(kind.label())
                     .checked(report.attached.contains(&kind))
                     .on_click(move |checked, _, app| {
                         entity.update(app, |report, cx| report.set_attached(kind, *checked, cx));
                     })
}

/// The chosen screenshots by name, each with a remove control, the add
/// control, and a note when the last choice skipped files.
fn screenshots_section(entity: &Entity<BugReport>, report: &BugReport, app: &App)
                       -> impl IntoElement {
    let muted = app.theme().muted_foreground;
    let add = {
        let entity = entity.clone();
        Button::new("add-screenshots").icon(IconName::Plus)
                                      .label(knot_core::l10n::t("bug_report.screenshots.add"))
                                      .ghost()
                                      .small()
                                      .disabled(report.screenshots.len()
                                                >= BUG_REPORT_MAX_SCREENSHOTS)
                                      .on_click(move |_: &ClickEvent, _, app| {
                                          entity.update(app, |report, cx| {
                                                    report.choose_screenshots(cx)
                                                });
                                      })
    };
    let rows = report.screenshots.iter().enumerate().map(|(index, path)| {
        let name = screenshots::file_name(path);
        let label = knot_core::l10n::t_with("bug_report.screenshots.remove", &[("name", &name)]);
        let entity = entity.clone();
        let path = path.clone();
        h_flex().gap_1()
                .child(Button::new(SharedString::from(format!("remove-screenshot-{index}")))
                           .icon(IconName::Close)
                           .ghost()
                           .xsmall()
                           .accessibility_label(label.clone())
                           .tooltip(label)
                           .on_click(move |_: &ClickEvent, _, app| {
                               entity.update(app, |report, cx| report.remove_screenshot(&path, cx));
                           }))
                .child(div().min_w_0().truncate().text_sm().child(name))
    });
    let skipped = (report.skipped > 0).then(|| {
                                          let files = knot_core::l10n::pluralize(report.skipped
                                                                                 as u64,
                                                                                 "count.file",
                                                                                 "count.files");
                                          div().text_xs()
             .text_color(muted)
             .child(knot_core::l10n::t_with("bug_report.screenshots.skipped",
                                            &[("files", &files),
                                              ("max", &BUG_REPORT_MAX_SCREENSHOTS.to_string())]))
                                      });
    v_flex().gap_1()
            .children(rows)
            .child(h_flex().child(add))
            .children(skipped)
}

/// Cancel and Report, anchored bottom-right. Cancel closes the window and
/// discards the report, as Escape does; Report files it.
pub(super) fn report_footer(entity: &Entity<BugReport>, app: &App) -> impl IntoElement {
    let report = entity.read(app);
    let enabled = report.can_report(app);
    let submitting = report.phase == super::form::Phase::Submitting;
    let submit = entity.clone();
    h_flex().flex_shrink_0()
            .justify_end()
            .gap_2()
            .child(div().debug_selector(|| "cancel-bug-report".into())
                        .child(Button::new("cancel-bug-report").label(knot_core::l10n::t("bug_report.cancel"))
                                                               .on_click(|_: &ClickEvent, window, _| {
                                                                   window.remove_window();
                                                               })))
            .child(Button::new("submit-bug-report").label(knot_core::l10n::t("bug_report.report"))
                                                   .primary()
                                                   .loading(submitting)
                                                   .disabled(!enabled)
                                                   .on_click(move |_: &ClickEvent, window, app| {
                                                       submit.update(app, |report, cx| {
                                                                 report.submit(window, cx)
                                                             });
                                                   }))
}

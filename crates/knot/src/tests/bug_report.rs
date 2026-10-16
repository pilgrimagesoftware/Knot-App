//! Help > Report an Issue opens one report window of its own (#39), and the
//! window keeps or files the report as `bug-reporting`'s spec says.
//!
//! The menu path is driven through `TestAppContext::dispatch_action`, which
//! runs the handler inside the window's update as macOS does - the reason
//! the open is deferred. The services are stubbed, so nothing here runs `gh`
//! or opens a browser.

use std::sync::Arc;

use gpui_kit::component::Root;
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, Entity, IntoElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, WindowOptions, div,
};
use knot_forge::ForgeAvailability;
use parking_lot::Mutex;

use crate::app_bootstrap::ReportIssue;
use crate::bug_report::{
    Attachment, BugReport, Diagnostics, IssueKind, LogKind, Outcome, Report, ReportServices,
    open_report, open_report_window_handle, register_report_issue_action_with,
};

/// A root view with no content, standing in for the window that was active
/// when the report was opened.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

/// What the stub submission answers, and every report it was handed.
#[derive(Clone)]
struct Stub {
    outcome:   Arc<Mutex<Outcome>>,
    submitted: Arc<Mutex<Vec<Report>>>,
}

impl Stub {
    fn answering(outcome: Outcome) -> Self {
        Self { outcome:   Arc::new(Mutex::new(outcome)),
               submitted: Arc::new(Mutex::new(Vec::new())), }
    }

    fn services(&self) -> ReportServices {
        let outcome = Arc::clone(&self.outcome);
        let submitted = Arc::clone(&self.submitted);
        ReportServices { collect:  Arc::new(|| {
                             Diagnostics { app:   "Knot 1.0.0 (2026-09-25, abc)".to_owned(),
                                           os:    "macOS 26.0".to_owned(),
                                           arch:  "aarch64".to_owned(),
                                           forge: ForgeAvailability::Ready, }
                         }),
                         submit:   Arc::new(move |report, _| {
                             submitted.lock().push(report.clone());
                             outcome.lock().clone()
                         }),
                         read_log: Arc::new(|kind| Attachment { kind,
                                                                path: None,
                                                                tail: Some(format!("{kind:?} log tail")) }), }
    }
}

fn app_with_one_window(cx: &mut TestAppContext, stub: &Stub) -> AnyWindowHandle {
    cx.update(|cx| {
          gpui_kit::init(cx);
          register_report_issue_action_with(stub.services(), cx);
          cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Blank);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open the test window")
            .into()
      })
}

/// Opens the report from the menu over one ordinary window, and returns a
/// context on the report window itself.
fn open_dialog(cx: &mut TestAppContext, stub: &Stub) -> (VisualTestContext, Entity<BugReport>) {
    let handle = app_with_one_window(cx, stub);
    cx.dispatch_action(handle, ReportIssue);
    cx.run_until_parked();
    let report = cx.update(|cx| open_report(cx))
                   .expect("Report an Issue opened no window");
    let report_window = cx.update(|cx| open_report_window_handle(cx))
                          .expect("the report has no window");
    assert_ne!(report_window, handle,
               "the report opened on the active window");
    (VisualTestContext::from_window(report_window, cx), report)
}

/// Whether the report window is still open.
///
/// Asked of the app rather than the window: once the window has closed, a
/// window-scoped update has nothing to run against.
fn dialog_is_open(cx: &mut VisualTestContext) -> bool {
    cx.cx.update(|cx| {
             open_report_window_handle(cx).is_some_and(|handle| cx.windows().contains(&handle))
         })
}

fn fill(cx: &mut VisualTestContext, report: &Entity<BugReport>, subject: &str, description: &str) {
    report.update_in(cx, |report, window, cx| {
              report.subject.update(cx, |input, cx| {
                                input.set_value(subject.to_owned(), window, cx)
                            });
              report.description.update(cx, |input, cx| {
                                    input.set_value(description.to_owned(), window, cx)
                                });
          });
    cx.run_until_parked();
}

fn submit(cx: &mut VisualTestContext, report: &Entity<BugReport>) {
    report.update_in(cx, |report, window, cx| report.submit(window, cx));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_menu_opens_a_window_of_its_own(cx: &mut TestAppContext) {
    let (mut cx, _report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));

    assert!(dialog_is_open(&mut cx), "Report an Issue opened no window");
    assert_eq!(cx.update(|_, cx| cx.windows().len()),
               2,
               "the report is a second window beside the first");
    assert!(cx.debug_bounds("bug-report-form").is_some(),
            "the form never reached the screen");
}

#[gpui_kit::test]
fn choosing_the_item_again_keeps_the_one_window(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let handle = app_with_one_window(cx, &stub);
    cx.dispatch_action(handle, ReportIssue);
    cx.run_until_parked();
    let first = cx.update(|cx| open_report(cx)).expect("no window opened");

    cx.dispatch_action(handle, ReportIssue);
    cx.run_until_parked();

    let second = cx.update(|cx| open_report(cx))
                   .expect("the window went away");
    assert_eq!(first.entity_id(),
               second.entity_id(),
               "a second report opened");
    assert_eq!(cx.update(|cx| cx.windows().len()),
               2,
               "a second report window opened");
}

#[gpui_kit::test]
fn the_diagnostics_are_filled_in_on_open(cx: &mut TestAppContext) {
    let (mut cx, report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));

    let text = cx.update(|_, cx| report.read(cx).diagnostics.read(cx).value().to_string());
    assert!(text.contains("Knot 1.0.0") && text.contains("aarch64"),
            "{text}");
}

/// #270: the diagnostics the issue carries say nothing about the reporter's
/// `gh`; the window says how the report will be sent beside its buttons.
#[gpui_kit::test]
fn how_the_report_is_sent_is_shown_not_filed(cx: &mut TestAppContext) {
    let (mut cx, report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));

    let (text, forge) = cx.update(|_, cx| {
                              let report = report.read(cx);
                              (report.diagnostics.read(cx).value().to_string(),
                               report.forge.clone())
                          });
    assert!(!text.contains(&knot_core::l10n::t("bug_report.delivery.gh"))
            && !text.contains("GitHub CLI"),
            "the filed diagnostics mention the forge: {text}");
    assert_eq!(forge,
               Some(ForgeAvailability::Ready),
               "the probe's answer should reach the window");
    assert!(cx.debug_bounds("bug-report-delivery").is_some(),
            "the delivery note is not drawn");
}

#[gpui_kit::test]
fn escape_cancels_and_files_nothing(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");
    drop(report);

    cx.simulate_keystrokes("escape");
    cx.run_until_parked();

    assert!(!dialog_is_open(&mut cx), "escape left the window open");
    assert!(stub.submitted.lock().is_empty(), "escape filed a report");
    assert!(cx.cx.update(|cx| open_report(cx)).is_none(),
            "the closed window's report outlived it");
}

#[gpui_kit::test]
fn report_needs_both_fields(cx: &mut TestAppContext) {
    let (mut cx, report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));

    fill(&mut cx, &report, "Crash", "   ");
    assert!(!cx.update(|_, cx| report.read(cx).can_report(cx)));

    fill(&mut cx, &report, "Crash", "It crashed.");
    assert!(cx.update(|_, cx| report.read(cx).can_report(cx)));
}

#[gpui_kit::test]
fn a_filed_issue_closes_the_window(cx: &mut TestAppContext) {
    let url = "https://github.com/pilgrimagesoftware/Knot-App/issues/7".to_owned();
    let stub = Stub::answering(Outcome::Filed(url));
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(!dialog_is_open(&mut cx),
            "a filed report left the window open");
    let submitted = stub.submitted.lock();
    assert_eq!(submitted.len(), 1);
    assert_eq!(submitted[0].subject, "Crash");
    assert_eq!(submitted[0].description, "It crashed.");
    assert!(submitted[0].diagnostics.contains("Knot 1.0.0"));
    assert!(!submitted[0].diagnostics.contains("GitHub CLI"),
            "the filed diagnostics mention the forge (#270)");
}

#[gpui_kit::test]
fn a_failure_keeps_the_report_and_allows_a_retry(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::FileFailed("HTTP 403".to_owned()));
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(dialog_is_open(&mut cx),
            "a failed submission closed the window");
    let (subject, description, retry) = cx.update(|_, cx| {
                                              let report = report.read(cx);
                                              (report.subject.read(cx).value().to_string(),
                                               report.description.read(cx).value().to_string(),
                                               report.can_report(cx))
                                          });
    assert_eq!((subject.as_str(), description.as_str()),
               ("Crash", "It crashed."));
    assert!(retry, "Report stayed disabled after a failure");

    submit(&mut cx, &report);
    assert_eq!(stub.submitted.lock().len(),
               2,
               "the retry was not submitted");
}

fn assert_hand_off_keeps_the_window(cx: &mut TestAppContext, outcome: Outcome) {
    let stub = Stub::answering(outcome.clone());
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(dialog_is_open(&mut cx), "{outcome:?} closed the window");
    let subject = cx.update(|_, cx| report.read(cx).subject.read(cx).value().to_string());
    assert_eq!(subject, "Crash", "{outcome:?} lost the subject");
}

#[gpui_kit::test]
fn a_browser_hand_off_keeps_the_window_open(cx: &mut TestAppContext) {
    assert_hand_off_keeps_the_window(cx, Outcome::BrowserReady);
}

#[gpui_kit::test]
fn a_browser_that_will_not_open_keeps_the_window_open(cx: &mut TestAppContext) {
    assert_hand_off_keeps_the_window(cx, Outcome::BrowserFailed);
}

/// A window of its own needs nothing to attach to (#39).
#[gpui_kit::test]
fn with_no_window_the_item_still_opens_the_report(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    cx.update(|cx| {
          gpui_kit::init(cx);
          register_report_issue_action_with(stub.services(), cx);
          cx.dispatch_action(&ReportIssue);
      });
    cx.run_until_parked();

    assert!(cx.update(|cx| open_report(cx)).is_some());
    assert_eq!(cx.update(|cx| cx.windows().len()), 1);
}

/// Cancel closes the window and files nothing, as Escape does.
#[gpui_kit::test]
fn cancel_closes_the_window_and_files_nothing(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");
    drop(report);

    let cancel = cx.debug_bounds("cancel-bug-report")
                   .expect("no Cancel button on screen");
    cx.simulate_click(cancel.center(), gpui_kit::Modifiers::none());
    cx.run_until_parked();

    assert!(!dialog_is_open(&mut cx), "Cancel left the window open");
    assert!(stub.submitted.lock().is_empty(), "Cancel filed a report");
}

/// A log is the user's to share: none rides along unless it is chosen.
#[gpui_kit::test]
fn no_log_is_attached_by_default(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(stub.submitted.lock()[0].attachments.is_empty());
}

#[gpui_kit::test]
fn a_chosen_log_rides_along_and_an_unchosen_one_does_not(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");
    report.update(&mut cx, |report, cx| {
              report.set_attached(LogKind::Mcp, true, cx);
              report.set_attached(LogKind::App, true, cx);
              report.set_attached(LogKind::App, false, cx);
          });

    submit(&mut cx, &report);

    let submitted = stub.submitted.lock();
    let kinds: Vec<_> = submitted[0].attachments
                                    .iter()
                                    .map(|attachment| attachment.kind)
                                    .collect();
    assert_eq!(kinds, [LogKind::Mcp]);
    assert_eq!(submitted[0].attachments[0].tail.as_deref(),
               Some("Mcp log tail"));
}

#[gpui_kit::test]
fn a_report_starts_as_a_bug(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert_eq!(stub.submitted.lock()[0].kind, IssueKind::Bug);
}

/// A feature request files as one, and carries no log - not even one checked
/// before the switch, which the hidden section no longer shows.
#[gpui_kit::test]
fn a_feature_request_files_as_one_without_logs(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Tabs", "Let agents live in tabs.");
    report.update_in(&mut cx, |report, window, cx| {
              report.set_attached(LogKind::Mcp, true, cx);
              report.set_kind(IssueKind::FeatureRequest, window, cx);
          });

    submit(&mut cx, &report);

    let submitted = stub.submitted.lock();
    assert_eq!(submitted[0].kind, IssueKind::FeatureRequest);
    assert!(submitted[0].attachments.is_empty());
}

#[gpui_kit::test]
fn switching_the_kind_keeps_what_was_typed(cx: &mut TestAppContext) {
    let (mut cx, report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));
    fill(&mut cx, &report, "Tabs", "Let agents live in tabs.");

    report.update_in(&mut cx, |report, window, cx| {
              report.set_kind(IssueKind::FeatureRequest, window, cx);
              report.set_kind(IssueKind::Bug, window, cx);
          });

    let (subject, description) = cx.update(|_, cx| {
                                       let report = report.read(cx);
                                       (report.subject.read(cx).value().to_string(),
                                        report.description.read(cx).value().to_string())
                                   });
    assert_eq!((subject.as_str(), description.as_str()),
               ("Tabs", "Let agents live in tabs."));
}

/// Screenshots (#566) go with a feature request as well as a bug - unlike
/// the logs. Only images are kept, the rest are counted as skipped, and one
/// removed before filing is not sent.
#[gpui_kit::test]
fn chosen_screenshots_ride_along_and_a_removed_one_does_not(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Tabs", "Let agents live in tabs.");
    let shot = |name: &str| std::path::PathBuf::from(format!("/Users/me/Desktop/{name}"));
    report.update_in(&mut cx, |report, window, cx| {
              report.set_kind(IssueKind::FeatureRequest, window, cx);
              report.add_screenshots(vec![shot("tabs.png"), shot("notes.txt"), shot("mock.jpg")],
                                     cx);
          });
    let skipped = report.read_with(&cx, |report, _| report.skipped);
    assert_eq!(skipped, 1,
               "the text file is skipped, and the window says so");

    report.update(&mut cx, |report, cx| {
              report.remove_screenshot(&shot("mock.jpg"), cx)
          });
    submit(&mut cx, &report);

    let submitted = stub.submitted.lock();
    assert_eq!(submitted[0].kind, IssueKind::FeatureRequest);
    assert_eq!(submitted[0].screenshots, [shot("tabs.png")]);
}

#[gpui_kit::test]
fn a_report_starts_with_no_screenshots(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(stub.submitted.lock()[0].screenshots.is_empty());
}

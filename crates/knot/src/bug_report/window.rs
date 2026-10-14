//! The bug-report window: the action that opens it, the single-instance
//! handle, and the view that hosts the form (#39).
//!
//! A window of its own rather than a dialog on whichever window was active:
//! the report can be written beside the window it is about, moved, and kept
//! open while the user looks something up, and it opens with no other
//! window at all.

use gpui_kit::base::v_flex;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{ActiveTheme, Root};
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, Styled, TitlebarOptions, WeakEntity, Window,
    WindowBounds, WindowOptions, px, size,
};

use super::pane::{report_content, report_footer};
use super::report::{BugReport, ReportServices};
use crate::app_bootstrap::ReportIssue;
use crate::app_support::observe_system_appearance;
use crate::consts::{
    BUG_REPORT_WINDOW_HEIGHT, BUG_REPORT_WINDOW_MIN_HEIGHT, BUG_REPORT_WINDOW_WIDTH,
};

/// The open window, if there is one: the single-instance handle.
///
/// The report is held weakly, because the window owns it through its view.
/// Closing the window drops the view, the report with it, and so this stops
/// upgrading - no close observer has to clear it.
struct OpenReport {
    services: ReportServices,
    window:   Option<AnyWindowHandle>,
    report:   Option<WeakEntity<BugReport>>,
}

impl gpui_kit::Global for OpenReport {}

/// Registers the `ReportIssue` handler over the live services.
pub(crate) fn register_report_issue_action(cx: &mut App) {
    register_report_issue_action_with(ReportServices::live(), cx);
}

/// [`register_report_issue_action`], over supplied services.
pub(crate) fn register_report_issue_action_with(services: ReportServices, cx: &mut App) {
    cx.set_global(OpenReport { services,
                               window: None,
                               report: None });
    cx.on_action(|_: &ReportIssue, cx| open_report_window(cx));
}

/// Raises the open report window, or opens one.
///
/// Deferred, like `quit_guard::request_quit`: the menu dispatch runs inside
/// the active window's update, and the window opened here reads which one
/// that was. The single-instance check runs inside the deferral too, so two
/// quick menu picks cannot both pass it before either opens.
fn open_report_window(cx: &mut App) {
    cx.defer(|cx| {
          let open = cx.global::<OpenReport>();
          if let (Some(window), Some(report)) = (open.window, open.report.as_ref())
             && report.upgrade().is_some()
             && window.update(cx, |_, window, _| window.activate_window())
                      .is_ok()
          {
              return;
          }
          let services = cx.global::<OpenReport>().services.clone();
          let origin = cx.active_window();
          let mut opened = None;
          let result = cx.open_window(window_options(cx), |window, cx| {
                             observe_system_appearance(window);
                             let report = cx.new(|cx| BugReport::new(services, origin, window, cx));
                             opened = Some(report.downgrade());
                             report.update(cx, |report, cx| {
                                       report.subject
                                             .update(cx, |input, cx| input.focus(window, cx));
                                       report.collect(window, cx);
                                   });
                             let view = cx.new(|cx| ReportWindow { report,
                                                                   focus: cx.focus_handle() });
                             cx.new(|cx| Root::new(view, window, cx))
                         });
          match result {
              Ok(handle) => {
                  let open = cx.global_mut::<OpenReport>();
                  open.window = Some(handle.into());
                  open.report = opened;
              }
              Err(error) => eprintln!("bug report: the window did not open: {error}"),
          }
      });
}

/// A utility window with the report's purpose in the OS titlebar, tall
/// enough for the whole form at its default size; it scrolls when smaller.
fn window_options(cx: &App) -> WindowOptions {
    WindowOptions { titlebar: Some(TitlebarOptions { title:
                                                         Some(knot_core::l10n::t("bug_report.title")
                                                                  .into()),
                                                     ..Default::default() }),
                    window_bounds: Some(WindowBounds::centered(size(px(BUG_REPORT_WINDOW_WIDTH),
                                                                    px(BUG_REPORT_WINDOW_HEIGHT)),
                                                               cx)),
                    window_min_size: Some(size(px(BUG_REPORT_WINDOW_WIDTH),
                                               px(BUG_REPORT_WINDOW_MIN_HEIGHT))),
                    ..WindowOptions::default() }
}

/// The open window's report, if one is open.
#[cfg(test)]
pub(crate) fn open_report(cx: &App) -> Option<Entity<BugReport>> {
    cx.try_global::<OpenReport>()?.report.as_ref()?.upgrade()
}

/// The open report window's handle, if one is open.
#[cfg(test)]
pub(crate) fn open_report_window_handle(cx: &App) -> Option<AnyWindowHandle> {
    let open = cx.try_global::<OpenReport>()?;
    open.report.as_ref()?.upgrade()?;
    open.window
}

/// The window's root view: the form, scrolling, over a fixed button row.
struct ReportWindow {
    report: Entity<BugReport>,
    /// Holds focus when no field does, so Escape still reaches the window.
    focus:  FocusHandle,
}

impl Render for ReportWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if window.focused(cx).is_none() {
            window.focus(&self.focus.clone(), cx);
        }
        v_flex().track_focus(&self.focus)
                .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, _| {
                                   // Scoped to this window rather than an
                                   // app-wide binding, as About and Import do.
                                   // Discards what was typed, per the spec.
                                   if event.keystroke.key == "escape" {
                                       window.remove_window();
                                   }
                               }))
                .size_full()
                .bg(cx.theme().background)
                .child(v_flex().id("bug-report-form")
                               .debug_selector(|| "bug-report-form".into())
                               .flex_1()
                               .min_h_0()
                               .overflow_y_scrollbar()
                               .p_5()
                               .child(report_content(&self.report, cx)))
                .child(v_flex().px_5()
                               .pb_5()
                               .pt_3()
                               .child(report_footer(&self.report, cx)))
    }
}

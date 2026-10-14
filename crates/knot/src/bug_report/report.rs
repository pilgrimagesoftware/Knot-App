//! The report the window collects: what was typed and chosen, and the
//! blocking work behind it - collecting diagnostics, filing - run on the
//! background executor and landed back through `update_in`, which notifies
//! the entity and so reaches a frame without any poll.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::component::WindowExt;
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::component::notification::Notification;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Context, Entity, PathPromptOptions, Subscription, Window,
};
use knot_forge::ForgeAvailability;

use super::diagnostics::Diagnostics;
use super::form::{Phase, can_report, filed_message};
use super::kind::IssueKind;
use super::logs::{self, Attachment, LogKind};
use super::screenshots::{self, Handoff};
use super::submit::{self, Outcome, Report};
use crate::consts::BUG_REPORT_DESCRIPTION_ROWS;

type Collect = dyn Fn() -> Diagnostics + Send + Sync;
type Submit = dyn Fn(&Report, &ForgeAvailability) -> Outcome + Send + Sync;
type ReadLog = dyn Fn(LogKind) -> Attachment + Send + Sync;

/// The blocking operations the report window calls, behind a seam so the
/// window tests neither shell out to `gh`, open a browser, nor read the user's
/// logs.
#[derive(Clone)]
pub(crate) struct ReportServices {
    pub(crate) collect:  Arc<Collect>,
    pub(crate) submit:   Arc<Submit>,
    pub(crate) read_log: Arc<ReadLog>,
}

impl ReportServices {
    /// The real host, the real `gh`, the real browser.
    pub(super) fn live() -> Self {
        Self { collect:  Arc::new(Diagnostics::collect),
               submit:   Arc::new(|report, forge| {
                   submit::submit(report,
                                  forge,
                                  &knot_forge::GhRunner::new(),
                                  crate::open_in::open_url,
                                  crate::open_in::reveal_path)
               }),
               read_log: Arc::new(logs::read), }
    }
}

/// What the report window holds while it is open. Created with it and
/// dropped with it, so a report never outlives its window and the next one
/// starts empty.
pub(crate) struct BugReport {
    /// Bug or feature request. Starts as a bug, the more common report.
    pub(crate) kind:        IssueKind,
    pub(crate) subject:     Entity<InputState>,
    pub(crate) description: Entity<TextareaState>,
    pub(crate) diagnostics: Entity<TextareaState>,
    /// The logs to attach, in the order they are offered. None by default:
    /// a log is the user's to share.
    pub(crate) attached:    Vec<LogKind>,
    /// The screenshots to name, in the order chosen (#566). None by
    /// default, as with the logs.
    pub(crate) screenshots: Vec<PathBuf>,
    /// How many files the last choice skipped - not an image, or past the
    /// cap - so the window can say so rather than drop them silently.
    pub(crate) skipped:     usize,
    pub(super) phase:       Phase,
    services:               ReportServices,
    /// The window that was active when the report was opened, which shows
    /// the "filed" confirmation once the report window has closed.
    origin:                 Option<AnyWindowHandle>,
    _subscriptions:         Vec<Subscription>,
}

impl BugReport {
    pub(super) fn new(services: ReportServices, origin: Option<AnyWindowHandle>,
                      window: &mut Window, cx: &mut Context<Self>)
                      -> Self {
        let subject = cx.new(|cx| {
                            InputState::new(window, cx)
                                .placeholder(knot_core::l10n::t("bug_report.subject_placeholder"))
                        });
        let description = cx.new(|cx| {
                                TextareaState::new(window, cx)
                    .placeholder(knot_core::l10n::t("bug_report.description_placeholder"))
                    // Fixed height, scrolling within: see the constant.
                    .auto_grow(BUG_REPORT_DESCRIPTION_ROWS, BUG_REPORT_DESCRIPTION_ROWS)
                            });
        let diagnostics =
            cx.new(|cx| {
                  let mut state = TextareaState::new(window, cx).auto_grow(4, 6);
                  state.set_value(knot_core::l10n::t("bug_report.diagnostics_collecting"),
                                  window,
                                  cx);
                  state
              });
        // Report's enabled state follows the fields, so a keystroke has to
        // re-render the window rather than wait for an unrelated repaint.
        let subscriptions = vec![cx.subscribe(&subject, |_, _, event: &InputEvent, cx| {
                                       if matches!(event, InputEvent::Change) {
                                           cx.notify();
                                       }
                                   }),
                                 cx.subscribe(&description, |_, _, event: &InputEvent, cx| {
                                       if matches!(event, InputEvent::Change) {
                                           cx.notify();
                                       }
                                   })];
        Self { kind: IssueKind::default(),
               subject,
               description,
               diagnostics,
               attached: Vec::new(),
               screenshots: Vec::new(),
               skipped: 0,
               phase: Phase::Editing,
               services,
               origin,
               _subscriptions: subscriptions }
    }

    /// Whether Report can be chosen right now.
    pub(crate) fn can_report(&self, cx: &App) -> bool {
        can_report(&self.subject.read(cx).value(),
                   &self.description.read(cx).value(),
                   &self.phase)
    }

    /// Switches what the report files as, and the fields' hints with it.
    /// What was typed stays: a user who picked the wrong kind first should
    /// not have to type it again.
    pub(crate) fn set_kind(&mut self, kind: IssueKind, window: &mut Window,
                           cx: &mut Context<Self>) {
        self.kind = kind;
        self.subject.update(cx, |input, cx| {
                        input.set_placeholder(kind.subject_placeholder(), window, cx)
                    });
        self.description.update(cx, |input, cx| {
                            input.set_placeholder(kind.description_placeholder(), window, cx)
                        });
        cx.notify();
    }

    /// Adds `kind` to the attached logs, or takes it off.
    pub(crate) fn set_attached(&mut self, kind: LogKind, attached: bool, cx: &mut Context<Self>) {
        self.attached.retain(|&chosen| chosen != kind);
        if attached {
            self.attached.push(kind);
            self.attached
                .sort_by_key(|chosen| LogKind::ALL.iter().position(|kind| kind == chosen));
        }
        cx.notify();
    }

    /// Adds the images among `picked` to the screenshots, noting how many
    /// were skipped.
    pub(crate) fn add_screenshots(&mut self, picked: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.skipped = screenshots::add(&mut self.screenshots, picked);
        cx.notify();
    }

    /// Takes `path` off the screenshots. The skipped note goes with it: it
    /// described a choice the list no longer reflects.
    pub(crate) fn remove_screenshot(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.screenshots.retain(|chosen| chosen != path);
        self.skipped = 0;
        cx.notify();
    }

    /// Opens the native file picker, and adds what the user chooses.
    pub(super) fn choose_screenshots(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files:       true,
            directories: false,
            multiple:    true,
            prompt:      Some(knot_core::l10n::t("bug_report.screenshots.add").into()),
        });
        cx.spawn(async move |this, cx| {
              let Ok(Ok(Some(picked))) = receiver.await
              else {
                  return;
              };
              let _ = this.update(cx, |report, cx| report.add_screenshots(picked, cx));
          })
          .detach();
    }

    /// Fills the diagnostics pane off the main thread.
    pub(super) fn collect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let collect = Arc::clone(&self.services.collect);
        cx.spawn_in(window, async move |this, cx| {
              let diagnostics = cx.background_executor()
                                  .spawn(async move { collect() })
                                  .await;
              let _ = this.update_in(cx, |report, window, cx| {
                              report.show_diagnostics(&diagnostics, window, cx);
                          });
          })
          .detach();
    }

    fn show_diagnostics(&mut self, diagnostics: &Diagnostics, window: &mut Window,
                        cx: &mut Context<Self>) {
        let text = diagnostics.text();
        self.diagnostics
            .update(cx, |state, cx| state.set_value(text, window, cx));
        cx.notify();
    }

    /// Files the report, or hands it to the browser.
    ///
    /// Diagnostics are collected again rather than read back off the pane:
    /// the forge may have been signed in since the window opened, and the
    /// probe is what decides the path. The pane is refreshed with what was
    /// sent, so what the user sees is what the issue carries.
    pub(crate) fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_report(cx) {
            return;
        }
        self.phase = Phase::Submitting;
        cx.notify();
        let subject = self.subject.read(cx).value().to_string();
        let description = self.description.read(cx).value().to_string();
        let kind = self.kind;
        // A log checked before switching to a feature request is not sent:
        // the section is hidden, so the user can no longer see it is chosen.
        let attached = if kind.offers_logs() {
            self.attached.clone()
        }
        else {
            Vec::new()
        };
        let screenshots = self.screenshots.clone();
        let handoff = Handoff::for_report(screenshots.len(), crate::open_in::can_reveal_files());
        let services = self.services.clone();
        cx.spawn_in(window, async move |this, cx| {
              let (diagnostics, outcome) =
                  cx.background_executor()
                    .spawn(async move {
                        let diagnostics = (services.collect)();
                        let attachments = attached.into_iter()
                                                  .map(|kind| (services.read_log)(kind))
                                                  .collect();
                        let report = Report { kind,
                                              subject,
                                              description,
                                              diagnostics: diagnostics.text(),
                                              attachments,
                                              screenshots };
                        let outcome = (services.submit)(&report, &diagnostics.forge);
                        (diagnostics, outcome)
                    })
                    .await;
              let _ = this.update_in(cx, |report, window, cx| {
                              report.show_diagnostics(&diagnostics, window, cx);
                              report.finish(outcome, handoff, window, cx);
                          });
          })
          .detach();
    }

    fn finish(&mut self, outcome: Outcome, handoff: Handoff, window: &mut Window,
              cx: &mut Context<Self>) {
        self.phase = match outcome {
            Outcome::Filed(url) => {
                self.confirm_filed(filed_message(&url, handoff), cx);
                window.remove_window();
                Phase::Editing
            }
            Outcome::FileFailed(error) => Phase::Failed(error),
            Outcome::BrowserReady => Phase::BrowserReady(handoff),
            Outcome::BrowserFailed => Phase::BrowserFailed,
        };
        cx.notify();
    }

    /// Shows `message` on the window the report was opened from, once this
    /// one has gone: a confirmation in the window being closed would vanish
    /// with it. Deferred, because this runs inside the report window's
    /// update and the origin's has to be a separate one. With no origin
    /// left - it was closed, or there was none - the issue is still filed;
    /// only the toast is lost.
    fn confirm_filed(&self, message: String, cx: &mut Context<Self>) {
        let Some(origin) = self.origin
        else {
            return;
        };
        cx.defer(move |cx| {
              let _ = origin.update(cx, |_, window, cx| {
                                window.push_notification(Notification::success(message), cx);
                            });
          });
    }
}

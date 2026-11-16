//! The editor's Schedule and When Idle controls (#404).
//!
//! Contract: `openspec/changes/session-scheduling`, `agent-editor-ui`'s "The
//! agent editor sets a schedule and an idle policy". Neither is offered for a
//! shell agent. A schedule is refused while the agent has no startup prompt -
//! there would be nothing to run - and an invalid entry says what to fix, with
//! the next run shown for a valid one.

use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, IntoElement, ParentElement, Styled, Subscription,
    Window, div, px,
};
use knot_core::schedule::{
    IdleAction, IdlePolicy, IdleRule, Preset, Schedule, TimeOfDay, Weekday, WeekdaySet, next_run,
};

use super::AgentEditor;

/// Which kind of schedule the picker shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScheduleKind {
    None,
    Daily,
    Weekdays,
    Days,
    EveryHours,
    Cron,
}

impl ScheduleKind {
    const ALL: [Self; 6] = [Self::None,
                            Self::Daily,
                            Self::Weekdays,
                            Self::Days,
                            Self::EveryHours,
                            Self::Cron];

    fn label(self) -> String {
        knot_core::l10n::t(match self {
                               Self::None => "agent_editor.schedule_kind.none",
                               Self::Daily => "agent_editor.schedule_kind.daily",
                               Self::Weekdays => "agent_editor.schedule_kind.weekdays",
                               Self::Days => "agent_editor.schedule_kind.days",
                               Self::EveryHours => "agent_editor.schedule_kind.every_hours",
                               Self::Cron => "agent_editor.schedule_kind.cron",
                           })
    }

    fn takes_a_time(self) -> bool {
        matches!(self, Self::Daily | Self::Weekdays | Self::Days)
    }
}

/// Which idle setting the picker shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IdleKind {
    Inherit,
    Off,
    Deactivate,
    Reset,
}

impl IdleKind {
    const ALL: [Self; 4] = [Self::Inherit, Self::Off, Self::Deactivate, Self::Reset];

    fn label(self) -> String {
        knot_core::l10n::t(match self {
                               Self::Inherit => "agent_editor.idle_kind.inherit",
                               Self::Off => "agent_editor.idle_kind.off",
                               Self::Deactivate => "agent_editor.idle_kind.deactivate",
                               Self::Reset => "agent_editor.idle_kind.reset",
                           })
    }
}

/// What the scheduling controls hold.
pub(super) struct SchedulingFields {
    pub(super) schedule_kind: ScheduleKind,
    pub(super) days:          [bool; 7],
    time_input:               Entity<InputState>,
    hours_input:              Entity<InputState>,
    cron_input:               Entity<InputState>,
    pub(super) idle_kind:     IdleKind,
    idle_minutes_input:       Entity<InputState>,
    /// A schedule kept as text this build cannot read, so saving the agent
    /// does not erase it unless the user picks something else.
    unreadable:               Option<String>,
    _subscriptions:           Vec<Subscription>,
}

/// The values the controls hold, as the pure parsers take them.
#[derive(Debug, Clone, Copy)]
pub(super) struct ScheduleValues<'a> {
    pub(super) kind:  ScheduleKind,
    pub(super) time:  &'a str,
    pub(super) hours: &'a str,
    pub(super) cron:  &'a str,
    pub(super) days:  [bool; 7],
}

/// The schedule the controls describe, or the catalog key of what to fix.
pub(super) fn schedule_from(values: ScheduleValues<'_>) -> Result<Option<Schedule>, &'static str> {
    let time = || {
        values.time
              .trim()
              .parse::<TimeOfDay>()
              .map_err(|()| "agent_editor.schedule_error_time")
    };
    let preset = match values.kind {
        ScheduleKind::None => return Ok(None),
        ScheduleKind::Daily => Preset::Daily { at: time()? },
        ScheduleKind::Weekdays => Preset::Weekdays { at: time()? },
        ScheduleKind::Days => {
            let days = WeekdaySet::new(Weekday::ALL.into_iter()
                                                   .zip(values.days)
                                                   .filter(|(_, on)| *on)
                                                   .map(|(day, _)| day))
                       .ok_or("agent_editor.schedule_error_days")?;
            Preset::Days { days, at: time()? }
        }
        ScheduleKind::EveryHours => {
            let hours = values.hours
                              .trim()
                              .parse::<u8>()
                              .ok()
                              .filter(|hours| (1..24).contains(hours))
                              .ok_or("agent_editor.schedule_error_hours")?;
            Preset::EveryHours { hours }
        }
        ScheduleKind::Cron => {
            return Schedule::cron(values.cron).map(Some)
                                              .ok_or("agent_editor.schedule_error_cron");
        }
    };
    Ok(Some(Schedule::Preset(preset)))
}

/// The idle policy the controls describe, or the catalog key of what to fix.
pub(super) fn idle_policy_from(kind: IdleKind, minutes: &str) -> Result<IdlePolicy, &'static str> {
    let action = match kind {
        IdleKind::Inherit => return Ok(IdlePolicy::Inherit),
        IdleKind::Off => return Ok(IdlePolicy::Off),
        IdleKind::Deactivate => IdleAction::Deactivate,
        IdleKind::Reset => IdleAction::Reset,
    };
    minutes.trim()
           .parse::<u32>()
           .ok()
           .and_then(|minutes| IdleRule::new(minutes, action))
           .map(IdlePolicy::Custom)
           .ok_or("agent_editor.idle_error_minutes")
}

impl SchedulingFields {
    /// The controls for `schedule` and `policy` - an agent's own, or none
    /// and the default for a new one.
    pub(super) fn new(schedule: Option<&Schedule>, policy: IdlePolicy, window: &mut Window,
                      cx: &mut App)
                      -> Self {
        let mut kind = ScheduleKind::None;
        let mut days = [false; 7];
        let (mut time, mut hours, mut cron) = (String::new(), String::new(), String::new());
        let mut unreadable = None;
        match schedule {
            None => {}
            Some(Schedule::Preset(Preset::Daily { at })) => {
                kind = ScheduleKind::Daily;
                time = at.to_string();
            }
            Some(Schedule::Preset(Preset::Weekdays { at })) => {
                kind = ScheduleKind::Weekdays;
                time = at.to_string();
            }
            Some(Schedule::Preset(Preset::Days { days: set, at })) => {
                kind = ScheduleKind::Days;
                time = at.to_string();
                for (slot, day) in days.iter_mut().zip(Weekday::ALL) {
                    *slot = set.contains(day);
                }
            }
            Some(Schedule::Preset(Preset::EveryHours { hours: n })) => {
                kind = ScheduleKind::EveryHours;
                hours = n.to_string();
            }
            Some(Schedule::Cron(expression)) => {
                kind = ScheduleKind::Cron;
                cron = expression.clone();
            }
            Some(Schedule::Unreadable(text)) => unreadable = Some(text.clone()),
        }
        let (idle_kind, minutes) = match policy {
            IdlePolicy::Inherit => (IdleKind::Inherit, String::new()),
            IdlePolicy::Off => (IdleKind::Off, String::new()),
            IdlePolicy::Custom(rule) => (match rule.action() {
                                             IdleAction::Deactivate => IdleKind::Deactivate,
                                             IdleAction::Reset => IdleKind::Reset,
                                         },
                                         rule.after_minutes().to_string()),
        };
        let input = |value: String, placeholder: &str, window: &mut Window, cx: &mut App| {
            let placeholder = knot_core::l10n::t(placeholder);
            cx.new(|cx| {
                  InputState::new(window, cx).placeholder(placeholder)
                                             .default_value(value)
              })
        };
        Self { schedule_kind: kind,
               days,
               time_input: input(time, "agent_editor.schedule_time_placeholder", window, cx),
               hours_input: input(hours, "agent_editor.schedule_hours_placeholder", window, cx),
               cron_input: input(cron, "agent_editor.schedule_cron_placeholder", window, cx),
               idle_kind,
               idle_minutes_input: input(minutes,
                                         "agent_editor.idle_minutes_placeholder",
                                         window,
                                         cx),
               unreadable,
               _subscriptions: Vec::new() }
    }

    /// Repaint the editor as any field changes, so the preview, the error
    /// and the disabled Save button follow the typing.
    pub(super) fn subscribe(&mut self, window: &mut Window, cx: &mut Context<AgentEditor>) {
        for input in [&self.time_input,
                      &self.hours_input,
                      &self.cron_input,
                      &self.idle_minutes_input]
        {
            self._subscriptions.push(cx.subscribe_in(input,
                                                     window,
                                                     |_: &mut AgentEditor, _, event, _, cx| {
                                                         if matches!(event, InputEvent::Change) {
                                                             cx.notify();
                                                         }
                                                     }));
        }
    }

    fn values<'a>(&self, time: &'a str, hours: &'a str, cron: &'a str) -> ScheduleValues<'a> {
        ScheduleValues { kind: self.schedule_kind,
                         time,
                         hours,
                         cron,
                         days: self.days }
    }
}

impl AgentEditor {
    /// The schedule and idle policy to save, or the message saying what to
    /// fix. A shell agent saves neither.
    pub(super) fn submitted_scheduling(&self, cx: &App)
                                       -> Result<(Option<Schedule>, IdlePolicy), String> {
        if knot_core::agent_type::is_shell(&self.agent_type) {
            return Ok((None, IdlePolicy::Inherit));
        }
        let fields = &self.scheduling;
        let (time, hours, cron) = (fields.time_input.read(cx).value().to_string(),
                                   fields.hours_input.read(cx).value().to_string(),
                                   fields.cron_input.read(cx).value().to_string());
        let schedule = match (&fields.unreadable, fields.schedule_kind) {
            (Some(text), ScheduleKind::None) => Some(Schedule::Unreadable(text.clone())),
            _ => schedule_from(fields.values(&time, &hours, &cron)).map_err(knot_core::l10n::t)?,
        };
        if schedule.as_ref().is_some_and(Schedule::is_runnable)
           && self.submitted_startup_prompt(cx).is_none()
        {
            return Err(knot_core::l10n::t("agent_editor.schedule_error_no_prompt"));
        }
        let minutes = fields.idle_minutes_input.read(cx).value().to_string();
        let policy = idle_policy_from(fields.idle_kind, &minutes).map_err(knot_core::l10n::t)?;
        Ok((schedule, policy))
    }

    /// The Schedule and When Idle rows, or nothing for a shell agent.
    pub(super) fn scheduling_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        if knot_core::agent_type::is_shell(&self.agent_type) {
            return Vec::new();
        }
        let mut rows = vec![Self::dialog_row(knot_core::l10n::t("agent_editor.schedule"),
                                             self.schedule_controls(cx)).into_any_element()];
        let status = match self.submitted_scheduling(cx) {
            Ok((Some(schedule), _)) => {
                next_run(&schedule, &jiff::Zoned::now()).map(|next| {
                    let when = crate::schedule_text::when_text(&next, &jiff::Zoned::now());
                    div().text_xs()
                         .text_color(cx.theme().muted_foreground)
                         .child(knot_core::l10n::t_with("agent.schedule_next", &[("when", &when)]))
                })
            }
            Ok((None, _)) => None,
            Err(message) => Some(div().text_xs().text_color(cx.theme().danger).child(message)),
        };
        rows.extend(status.map(IntoElement::into_any_element));
        rows.push(Self::dialog_hint(cx, knot_core::l10n::t("agent_editor.schedule_hint"))
                      .into_any_element());
        rows.push(Self::dialog_row(knot_core::l10n::t("agent_editor.idle"),
                                   self.idle_controls(cx)).into_any_element());
        rows.push(Self::dialog_hint(cx, knot_core::l10n::t("agent_editor.idle_hint"))
                      .into_any_element());
        rows
    }

    fn schedule_controls(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let editor = cx.entity();
        let fields = &self.scheduling;
        let picker = Button::new("agent-schedule-kind").label(fields.schedule_kind.label())
                                                       .dropdown_caret(true)
                                                       .dropdown_menu(move |menu, _, _| {
                                                           let mut menu = menu;
                                                           for kind in ScheduleKind::ALL {
                                                               let editor = editor.clone();
                                                               menu = menu.item(PopupMenuItem::new(kind.label())
                                                                   .on_click(move |_, _, app| {
                                                                       editor.update(app, |e, cx| {
                                                                           e.scheduling.schedule_kind = kind;
                                                                           e.scheduling.unreadable = None;
                                                                           cx.notify();
                                                                       });
                                                                   }));
                                                           }
                                                           menu
                                                       });
        let kind = fields.schedule_kind;
        let day_toggles = (kind == ScheduleKind::Days).then(|| {
            let editor = cx.entity();
            h_flex().gap_1().children(Weekday::ALL.into_iter().enumerate().map(move |(index, day)| {
                let on = self.scheduling.days[index];
                let editor = editor.clone();
                let label = knot_core::l10n::t(&format!("schedule.weekday_short.{}", day.token()));
                let button = Button::new(("agent-schedule-day", index)).label(label).small();
                let button = if on { button.primary() } else { button.ghost() };
                button.on_click(move |_, _, app| {
                          editor.update(app, |e, cx| {
                                    e.scheduling.days[index] = !e.scheduling.days[index];
                                    cx.notify();
                                });
                      })
            }))
        });
        h_flex().gap_2()
                .items_center()
                .child(picker)
                .children(day_toggles)
                .children(kind.takes_a_time()
                              .then(|| Input::new(&self.scheduling.time_input).w(px(72.))))
                .children((kind == ScheduleKind::EveryHours)
                              .then(|| Input::new(&self.scheduling.hours_input).w(px(56.))))
                .children((kind == ScheduleKind::Cron)
                              .then(|| Input::new(&self.scheduling.cron_input).w(px(160.))))
    }

    fn idle_controls(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let editor = cx.entity();
        let kind = self.scheduling.idle_kind;
        let picker = Button::new("agent-idle-kind").label(kind.label())
                                                   .dropdown_caret(true)
                                                   .dropdown_menu(move |menu, _, _| {
                                                       let mut menu = menu;
                                                       for kind in IdleKind::ALL {
                                                           let editor = editor.clone();
                                                           menu = menu.item(PopupMenuItem::new(kind.label())
                                                               .on_click(move |_, _, app| {
                                                                   editor.update(app, |e, cx| {
                                                                       e.scheduling.idle_kind = kind;
                                                                       cx.notify();
                                                                   });
                                                               }));
                                                       }
                                                       menu
                                                   });
        h_flex().gap_2()
                .items_center()
                .child(picker)
                .children(matches!(kind, IdleKind::Deactivate | IdleKind::Reset).then(|| {
                    h_flex().gap_1()
                            .items_center()
                            .child(Input::new(&self.scheduling.idle_minutes_input).w(px(56.)))
                            .child(div().text_sm().child(knot_core::l10n::t("agent_editor.idle_minutes")))
                }))
    }
}

#[cfg(test)]
mod tests;

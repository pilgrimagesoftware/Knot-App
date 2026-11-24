//! The expanded (non-compact) agent row body: avatar tile, name, the detail
//! lines under it, and the state dot. The row's selection, drag, click and
//! menu wiring stays in `sidebar`; the compact body is in `sidebar_compact`.

use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use super::status_dot::DotDetail;
use super::status_dot::status_dot;
use crate::app_support::single_line;
use crate::settings_window::SettingsWindow;
use crate::workspace_window::DetailLineSize;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::detail_line;

/// What the expanded row body draws, moved out of the `agent_rows` loop as
/// one value because the body reads every field of the row snapshot.
pub(super) struct ExpandedAgentRow {
    pub(super) id:            Uuid,
    pub(super) avatar:        String,
    pub(super) name:          String,
    pub(super) folder_name:   String,
    pub(super) state:         knot_agents::AgentState,
    pub(super) idle_since:    Option<std::time::SystemTime>,
    pub(super) input_reason:  Option<String>,
    pub(super) session_limit: Option<knot_agents::SessionLimit>,
    pub(super) is_shell:      bool,
    pub(super) is_companion:  bool,
    pub(super) is_locked:     bool,
    pub(super) autopilot:     bool,
    pub(super) header_title:  String,
    pub(super) persona_name:  Option<String>,
    pub(super) agent_type:    String,
    pub(super) schedule_line: Option<String>,
    pub(super) idle_line:     Option<String>,
}

/// Builds the expanded row body for `row`, with its detail lines set in the
/// title font.
pub(super) fn expanded_agent_row_body(row: ExpandedAgentRow, title_font_name: &str,
                                      title_font_size: gpui_kit::Pixels,
                                      cx: &mut Context<WorkspaceWindow>)
                                      -> gpui_kit::AnyElement {
    let ExpandedAgentRow { id,
                           avatar,
                           name,
                           folder_name,
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
                           schedule_line,
                           idle_line, } = row;
    let title_font_name = title_font_name.to_owned();
    h_flex().w_full()
            .gap_3()
            .items_start()
            .child(div().w(px(40.))
                        .h(px(40.))
                        .flex_shrink_0()
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_2xl()
                        .child(avatar))
            .child(v_flex().flex_1()
                           .min_w_0()
                           .gap_0p5()
                           .child(div().w_full()
                                       .min_w_0()
                                       .overflow_hidden()
                                       .whitespace_nowrap()
                                       .text_ellipsis()
                                       .font_semibold()
                                       // A name is stored text, and
                                       // imported rosters carry
                                       // whatever the other tool held;
                                       // the flags above stop soft
                                       // wrapping only, so a name with
                                       // a line break in it would grow
                                       // the row.
                                       .child(single_line(&name)))
                           // A companion says so where a primary
                           // agent names its type - it has no
                           // coding-agent type of its own, and an
                           // unlabelled row gave no clue what it was.
                           .children(is_companion.then(|| {
                                                     detail_line(
                                            gpui_kit::assets::IconName::CornerDownRight,
                                            knot_core::l10n::t("agent.companion"),
                                            DetailLineSize::Small,
                                            title_font_name.clone(),
                                            title_font_size,
                                            cx,
                                        )
                                                 }))
                           // A locked agent is shut out of agent
                           // messaging (#310). Said on the row,
                           // because a message to it fails and the
                           // user should not have to open a menu to
                           // find out why.
                           .children(is_locked.then(|| {
                                                  detail_line(gpui_kit::assets::IconName::Lock,
                                                              knot_core::l10n::t("agent.locked"),
                                                              DetailLineSize::Small,
                                                              title_font_name.clone(),
                                                              title_font_size,
                                                              cx)
                                              }))
                           // Autopilot reads this agent's replies
                           // and may answer or flag them (#323).
                           .children(autopilot.then(|| {
                                                  detail_line(gpui_kit::assets::IconName::Fan,
                                                              knot_core::l10n::t("agent.autopilot"),
                                                              DetailLineSize::Small,
                                                              title_font_name.clone(),
                                                              title_font_size,
                                                              cx)
                                              }))
                           // When it next starts on its own, or a
                           // run it missed (#404).
                           .children(schedule_line.clone().map(|text| {
                                                              detail_line(
                                            gpui_kit::assets::IconName::CalendarClock,
                                            text,
                                            DetailLineSize::Small,
                                            title_font_name.clone(),
                                            title_font_size,
                                            cx,
                                        )
                                                          }))
                           // Near the end of an idle period only, so
                           // the sidebar does not fill with timers.
                           .children(idle_line.clone().map(|text| {
                                                          detail_line(
                                            gpui_kit::assets::IconName::Timer,
                                            text,
                                            DetailLineSize::Small,
                                            title_font_name.clone(),
                                            title_font_size,
                                            cx,
                                        )
                                                      }))
                           // The agent type reads as one of the
                           // row's detail lines, directly under the
                           // name and above the persona - not
                           // right-aligned opposite it, where it
                           // floated away from the name it
                           // describes and crowded the state dot.
                           .children((!is_shell).then(|| {
                                                    detail_line(
                                            SettingsWindow::agent_type_icon(&agent_type),
                                            SettingsWindow::agent_type_label(&agent_type)
                                                .to_string(),
                                            DetailLineSize::Small,
                                            title_font_name.clone(),
                                            title_font_size,
                                            cx,
                                        )
                                                }))
                           .children(persona_name.map(|persona_name| {
                                                     // A drawn icon, not the
                                                     // `👤` this line
                                                     // used to carry inside its
                                                     // text: an
                                                     // emoji keeps its own
                                                     // colour and size,
                                                     // so it was the one thing
                                                     // in the
                                                     // column that did not line
                                                     // up.
                                                     detail_line(gpui_kit::assets::IconName::User,
                                                                 persona_name,
                                                                 DetailLineSize::Small,
                                                                 title_font_name.clone(),
                                                                 title_font_size,
                                                                 cx)
                                                 }))
                           .child(detail_line(gpui_kit::assets::IconName::Activity,
                                              header_title,
                                              DetailLineSize::Body,
                                              title_font_name.clone(),
                                              title_font_size,
                                              cx))
                           .child(detail_line(gpui_kit::assets::IconName::Folder,
                                              folder_name,
                                              DetailLineSize::Body,
                                              title_font_name.clone(),
                                              title_font_size,
                                              cx)))
            // The state dot alone. The working indicator is
            // deliberately not here: beside the dot it says
            // the same thing twice, and the row already
            // carries the one thing the dot cannot - a
            // stopped agent's row is dimmed as a whole
            // (see `agent-list-ui`).
            .children((!is_shell).then(|| {
                                     status_dot(id,
                                                state,
                                                DotDetail { idle_since,
                                                            input_reason: input_reason.clone(),
                                                            session_limit: session_limit.clone() },
                                                crate::status_palette::status_palette(cx)).mt_1()
                                 }))
            .into_any_element()
}

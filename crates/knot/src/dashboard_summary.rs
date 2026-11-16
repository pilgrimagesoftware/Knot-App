//! The dashboards' status summary (#10, `dashboard-status-and-quick-prompt`):
//! how many agents are in each state, beside each section's name and, in the
//! Command Center, over every workspace in its header.
//!
//! Counted through `working_indicator::state`, the function that picks each
//! card's indicator, so the summary and the cards cannot disagree. Built from
//! the same snapshots the cards draw from: no reads of its own.

use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::rgb;

use crate::dashboard::DashboardAgent;
use crate::status_palette::StatusPalette;
use crate::working_indicator::WorkingIndicatorState;

/// The order states are listed in: what needs the user first.
const ORDER: [WorkingIndicatorState; 5] = [WorkingIndicatorState::AwaitingInput,
                                           WorkingIndicatorState::Working,
                                           WorkingIndicatorState::Idle,
                                           WorkingIndicatorState::Error,
                                           WorkingIndicatorState::Off];

/// How many of `agents` are in each state, in [`ORDER`], leaving out states
/// none is in. Shell agents are not counted: they have no agent state.
pub(crate) fn status_counts<'a>(agents: impl IntoIterator<Item = &'a DashboardAgent>)
                                -> Vec<(WorkingIndicatorState, usize)> {
    let mut counts = [0_usize; ORDER.len()];
    for agent in agents.into_iter().filter(|agent| !agent.is_shell) {
        let state = crate::working_indicator::state(agent.state, agent.has_session);
        if let Some(slot) = ORDER.iter().position(|candidate| *candidate == state) {
            counts[slot] += 1;
        }
    }
    ORDER.into_iter()
         .zip(counts)
         .filter(|(_, count)| *count > 0)
         .collect()
}

/// The catalog key for counting agents in `state`, read through
/// `l10n::t_count` (`{key}_one` / `{key}_other`), so the translator words the
/// whole entry and places the number.
pub(crate) fn label_key(state: WorkingIndicatorState) -> &'static str {
    match state {
        WorkingIndicatorState::AwaitingInput => "dashboard.summary.awaiting",
        WorkingIndicatorState::Working => "dashboard.summary.working",
        WorkingIndicatorState::Idle => "dashboard.summary.idle",
        WorkingIndicatorState::Error => "dashboard.summary.error",
        WorkingIndicatorState::Off => "dashboard.summary.off",
    }
}

/// A state's dot colour, the indicator's own.
fn dot_color(state: WorkingIndicatorState, palette: &StatusPalette) -> u32 {
    match state {
        WorkingIndicatorState::AwaitingInput => palette.input,
        WorkingIndicatorState::Working => palette.running,
        WorkingIndicatorState::Idle => palette.stopped,
        WorkingIndicatorState::Error => palette.error,
        WorkingIndicatorState::Off => palette.card_muted,
    }
}

/// The summary of `counts` (from [`status_counts`]): a coloured dot and a
/// count per state, or nothing when there is nothing to count.
pub(crate) fn summary(counts: Vec<(WorkingIndicatorState, usize)>,
                      palette: &'static StatusPalette)
                      -> Option<impl IntoElement + use<>> {
    (!counts.is_empty()).then(|| {
                            h_flex().gap_3().text_sm().children(counts.into_iter().map(
            |(state, count)| {
                h_flex().gap_1()
                        .items_center()
                        .child(div().size(px(8.))
                                    .rounded_full()
                                    .bg(rgb(dot_color(state, palette))))
                        .child(knot_core::l10n::t_count(label_key(state), count as u64, &[]))
            },
        ))
                        })
}

#[cfg(test)]
mod tests;

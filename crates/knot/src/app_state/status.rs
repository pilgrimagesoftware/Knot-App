//! How an agent's state reads on screen: its label, tooltip, colour, the
//! session-limit badge and the diff-stats row. Pure presentation over the
//! store's values; nothing here reads or writes the store.

use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::div;
use gpui_kit::rgb;

use crate::status_palette::StatusPalette;

/// User-facing label for the agent's automatic state-machine state, matching
/// the Swift reference's raw strings (not the Rust enum names).
pub(crate) fn state_label(state: knot_agents::AgentState) -> &'static str {
    match state {
        knot_agents::AgentState::Idle => "Idle",
        knot_agents::AgentState::Running => "Working",
        knot_agents::AgentState::Input => "Awaiting input",
        knot_agents::AgentState::Error => "Error",
    }
}

/// What a status surface calls the agent's state: [`state_label`], except
/// that an agent stopped by its session's usage limit says so instead of
/// "Error" (#318), with when it lifts when `with_reset` and the provider
/// said. `limit` is [`knot_agents::Agent::active_session_limit`].
pub(crate) fn status_label(state: knot_agents::AgentState,
                           limit: Option<&knot_agents::SessionLimit>, with_reset: bool)
                           -> String {
    match (state, limit) {
        (knot_agents::AgentState::Error, Some(limit)) => {
            match limit.resets.as_deref().filter(|_| with_reset) {
                Some(resets) => knot_core::l10n::t_with("sidebar.status_session_limit_resets",
                                                        &[("resets", resets)]),
                None => knot_core::l10n::t("sidebar.status_session_limit").to_string(),
            }
        }
        _ => state_label(state).to_string(),
    }
}

/// "Resets <when>", for a line beside a short "Session limit" label.
pub(crate) fn session_limit_resets(limit: &knot_agents::SessionLimit) -> Option<String> {
    limit.resets
         .as_deref()
         .map(|resets| {
             knot_core::l10n::t_with("sidebar.session_limit_resets", &[("resets", resets)])
         })
}

/// The sidebar status dot's tooltip (#582): the header's label for `state`,
/// and for an idle agent that has finished a message in this run, when it
/// did. Called when the tooltip opens, so "5m ago" is never stale.
///
/// An agent waiting on the user says why, when something said (#344):
/// `input_reason` is [`knot_agents::Agent::input_reason`]. One stopped by a
/// usage limit says so, and when it lifts (#318).
pub(crate) fn state_tooltip(state: knot_agents::AgentState,
                            idle_since: Option<std::time::SystemTime>,
                            input_reason: Option<&str>,
                            session_limit: Option<&knot_agents::SessionLimit>)
                            -> String {
    if let (knot_agents::AgentState::Error, Some(_)) = (state, session_limit) {
        return status_label(state, session_limit, true);
    }
    let label = state_label(state);
    if let (knot_agents::AgentState::Input, Some(reason)) = (state, input_reason) {
        return awaiting_because(label, reason);
    }
    match (state, idle_since) {
        (knot_agents::AgentState::Idle, Some(at)) => {
            knot_core::l10n::t_with("sidebar.status_idle_since",
                                    &[("state", label),
                                      ("when", &crate::timestamp::relative_timestamp(at)),
                                      ("at", &crate::timestamp::absolute_timestamp(at))])
        }
        _ => label.to_string(),
    }
}

/// "Awaiting input: <reason>", for the dot's tooltip and the dashboard
/// card's status line.
pub(crate) fn awaiting_because(label: &str, reason: &str) -> String {
    knot_core::l10n::t_with("sidebar.status_awaiting_because",
                            &[("state", label), ("reason", reason)])
}

/// A diff stat with only its figures colored - additions green, deletions
/// red, the changed-file count blue - and the words and brackets muted, so
/// the numbers are what the eye lands on. The figures take `palette`'s
/// idle, error and input colours, the same as [`state_color`]'s, in the
/// shade for the active appearance (#12). Shared by the workspace header
/// and the dashboard cards so the two can't drift apart; the caller styles
/// the row's font and size.
///
/// The file noun comes from `l10n::plural_noun` rather than a local
/// `if count == 1`, and separately from the count so only the number takes
/// the accent color.
///
/// The only `whitespace_nowrap` in the crate that does not pass its text
/// through [`crate::app_support::single_line`], and it needs no such pass:
/// every string here is formatted from integers Knot holds, so none can
/// arrive carrying a line break.
pub(crate) fn diff_stats_row(stats: &knot_git::DiffStats, muted: gpui_kit::Hsla,
                             palette: &StatusPalette)
                             -> gpui_kit::Div {
    let files = knot_core::l10n::plural_noun(stats.files_changed, "count.file", "count.files");
    h_flex().flex_shrink_0()
            .whitespace_nowrap()
            .text_color(muted)
            .gap_1()
            .items_baseline()
            .child(div().text_color(rgb(palette.idle))
                        .child(format!("+{}", stats.insertions)))
            .child(div().text_color(rgb(palette.error))
                        .child(format!("-{}", stats.deletions)))
            .child(h_flex().items_baseline()
                           .child(div().child("("))
                           .child(div().text_color(rgb(palette.input))
                                       .child(stats.files_changed.to_string()))
                           .child(div().ml_1().child(format!("{files})"))))
}

/// The status dot's colour: [`state_color`], except that an agent stopped by
/// its session's usage limit gets the palette's stronger `session_limit` red
/// rather than `error`'s, which reads as peach on dark (#369). `limit` is
/// [`knot_agents::Agent::active_session_limit`].
pub(crate) fn status_color(state: knot_agents::AgentState,
                           limit: Option<&knot_agents::SessionLimit>, palette: &StatusPalette)
                           -> gpui_kit::Hsla {
    match (state, limit) {
        (knot_agents::AgentState::Error, Some(_)) => rgb(palette.session_limit).into(),
        _ => state_color(state, palette),
    }
}

/// The "Session limit" label as a badge: bold white text on the palette's
/// `session_limit_fill` (#369). A badge rather than red text, because no
/// saturated red reaches text contrast on every dark surface, while white on
/// that fill does.
pub(crate) fn session_limit_badge(text: impl Into<gpui_kit::SharedString>,
                                  palette: &StatusPalette)
                                  -> gpui_kit::Div {
    use gpui_kit::{ParentElement, Styled};
    gpui_kit::div().px_1p5()
                   .rounded_md()
                   .bg(rgb(palette.session_limit_fill))
                   .text_color(gpui_kit::white())
                   .font_weight(gpui_kit::FontWeight::BOLD)
                   .child(text.into())
}

/// Status-dot color for the agent's automatic state. Diverges from the
/// Swift reference (which uses red for both input and error) by giving
/// "awaiting input" its own blue, since it isn't a failure state.
/// The shade is `palette`'s, for the active appearance (#12).
pub(crate) fn state_color(state: knot_agents::AgentState, palette: &StatusPalette)
                          -> gpui_kit::Hsla {
    match state {
        knot_agents::AgentState::Idle => rgb(palette.idle).into(),
        knot_agents::AgentState::Running => rgb(palette.running).into(),
        knot_agents::AgentState::Input => rgb(palette.input).into(),
        knot_agents::AgentState::Error => rgb(palette.error).into(),
    }
}

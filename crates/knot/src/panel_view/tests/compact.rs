//! Tests for compact mode: a run of tool calls drawn as one summary row.

use super::fixture::*;
use crate::panel_view::summary_row::*;

/// "Normal rendering remains available": with the preference off every
/// message draws as itself, tool calls included.
#[test]
fn compact_mode_off_draws_every_message_as_itself() {
    let state = conversation_with_a_run();
    for index in 0..state.messages.len() {
        assert_eq!(compact_row(index, &state, false),
                   CompactRow::Message,
                   "message {index} should render normally with compact mode off");
    }
}

/// "Compact rendering": with the preference on, the run's first call
/// carries the summary and the rest draw nothing. Non-tool messages are
/// untouched either way.
#[test]
fn compact_mode_on_summarizes_the_run_and_covers_the_rest() {
    let state = conversation_with_a_run();
    assert_eq!(compact_row(0, &state, true),
               CompactRow::Message,
               "the prompt");
    assert_eq!(compact_row(1, &state, true),
               CompactRow::Summary,
               "the run head");
    assert_eq!(compact_row(2, &state, true),
               CompactRow::Covered,
               "covered by it");
    assert_eq!(compact_row(3, &state, true),
               CompactRow::Message,
               "the text after");
}

/// "Disable compact mode": switching modes changes only what is drawn.
/// The same messages are still there, in the same order, with the same
/// tool-call records - nothing was consumed by either rendering.
#[test]
fn switching_modes_leaves_the_underlying_messages_untouched() {
    let state = conversation_with_a_run();
    let before = state.messages.clone();

    for index in 0..state.messages.len() {
        let _ = compact_row(index, &state, true);
        let _ = compact_row(index, &state, false);
    }

    assert_eq!(state.messages, before);
}

/// Opening a run in compact mode puts every one of its calls back to
/// individual cards - the inspection path compact mode has to keep.
#[test]
fn an_opened_run_draws_its_cards_again() {
    let mut state = conversation_with_a_run();
    state.toggle_tool_run("a".to_string());

    assert_eq!(compact_row(1, &state, true), CompactRow::Message);
    assert_eq!(compact_row(2, &state, true), CompactRow::Message);
}

/// The summary line's copy comes from the catalogue. Asserted as "the key
/// resolves", never as the English wording, so a copy edit cannot fail
/// this.
#[test]
fn every_summary_phrase_resolves_in_the_catalogue() {
    let keys = ["panel.summary.calls_one",
                "panel.summary.calls_other",
                "panel.summary.files_edited_one",
                "panel.summary.files_edited_other",
                "panel.summary.files_read_one",
                "panel.summary.files_read_other",
                "panel.summary.commands_run_one",
                "panel.summary.commands_run_other",
                "panel.summary.failures_one",
                "panel.summary.failures_other",
                "panel.summary.separator",
                "panel.summary.running_suffix",
                "settings.compact_tool_calls",
                "settings.compact_tool_calls_hint"];
    for key in keys {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalogue");
    }
}

/// The counts a run actually has appear in the line; the ones it does not
/// are omitted rather than shown as zero. Checked by substring against the
/// catalogue's own rendering, not against fixed English.
#[test]
fn the_summary_line_omits_counts_the_run_does_not_have() {
    let summary = crate::panel_state::ToolRunSummary { calls: 3,
                                                       files_read: 2,
                                                       ..Default::default() };
    let line = summary_text(summary);

    assert!(line.contains('3'), "the call count is always shown: {line}");
    assert!(line.contains('2'), "a count the run has is shown: {line}");
    assert!(!line.contains('0'),
            "a count it does not have is omitted: {line}");
}

/// A failed call keeps its own count in the line, so compact mode never
/// hides that something went wrong.
#[test]
fn a_failure_is_named_in_the_summary_line() {
    let quiet = summary_text(crate::panel_state::ToolRunSummary { calls: 2,
                                                                  ..Default::default() });
    let failed = summary_text(crate::panel_state::ToolRunSummary { calls: 2,
                                                                   failed: 1,
                                                                   ..Default::default() });

    assert_ne!(quiet, failed, "a failure has to change the line");
    assert!(failed.len() > quiet.len(),
            "it adds to it: {failed:?} vs {quiet:?}");
}
